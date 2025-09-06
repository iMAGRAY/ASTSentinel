use anyhow::{Context, Result};
use std::io::{self, Read};
use tokio;
use serde_json;
use serde::{Deserialize, Serialize};
use tiktoken_rs::get_bpe_from_model;
use lru::LruCache;
use lazy_static::lazy_static;
use std::sync::Mutex;
use sha2::{Sha256, Digest};
use std::num::NonZeroUsize;

// Use universal AI client for multi-provider support
use rust_validation_hooks::providers::ai::UniversalAIClient;
use rust_validation_hooks::*;

/// Memory organization hook for conversation stop event
/// Processes and optimizes conversation memory using GPT-5-nano
#[derive(Debug, Serialize, Deserialize)]
struct MemoryHookInput {
    #[serde(rename = "hookEvent")]
    hook_event: String,
    
    #[serde(rename = "conversationId")]
    conversation_id: Option<String>,
    
    #[serde(rename = "transcriptPath")]
    transcript_path: Option<String>,
    
    #[serde(rename = "memoryPath")]
    memory_path: Option<String>,
    
    #[serde(rename = "contextWindow")]
    context_window: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryHookOutput {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: MemoryHookSpecificOutput,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryHookSpecificOutput {
    #[serde(rename = "hookEventName")]
    hook_event_name: String,
    
    #[serde(rename = "memoryOptimized")]
    memory_optimized: bool,
    
    #[serde(rename = "tokensReduced")]
    tokens_reduced: Option<usize>,
    
    #[serde(rename = "summary")]
    summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryOptimization {
    optimized_memories: Vec<Memory>,
    total_tokens: usize,
    reduction_ratio: f64,
    key_insights: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Memory {
    timestamp: String,
    category: String,
    content: String,
    relevance_score: f64,
}

/// Read conversation transcript from file
async fn read_transcript(path: &str) -> Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read transcript from {}", path))
}

/// Read existing memory file
async fn read_memory(path: &str) -> Result<String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(content),
        Err(_) => Ok(String::new()), // Return empty if file doesn't exist
    }
}

/// Write optimized memory back to file
async fn write_memory(path: &str, content: &str) -> Result<()> {
    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("Failed to write memory to {}", path))
}

/// Load memory optimization prompt
async fn load_memory_prompt() -> Result<String> {
    let prompt_path = get_prompts_dir().join("memory_optimization.txt");
    Ok(tokio::fs::read_to_string(prompt_path)
        .await
        .unwrap_or_else(|_| {
            // Fallback prompt if file doesn't exist
            String::from(
                "You are a memory optimization system. Your task is to process conversation history and extract key information.

CRITICAL INSTRUCTIONS:
- Extract only essential facts, decisions, and context
- Remove redundant information and verbose explanations
- Organize memories by category (tasks, solutions, errors, decisions)
- Keep technical details precise and actionable
- Each memory should be self-contained and immediately useful
- Prioritize recent and frequently referenced information
- Remove pleasantries, filler text, and repetitions
- Format as structured, searchable entries

MEMORY CATEGORIES:
1. TASKS: What was requested and completed
2. SOLUTIONS: Technical solutions and fixes applied
3. ERRORS: Problems encountered and how they were resolved
4. DECISIONS: Architectural and design choices made
5. CONTEXT: Project-specific information and constraints
6. TOOLS: Commands, APIs, and tools used

OUTPUT FORMAT:
Return a JSON object with optimized_memories array, where each memory has:
- timestamp: when it occurred
- category: one of the categories above
- content: concise, actionable description
- relevance_score: 0.0 to 1.0 based on importance

Focus on creating memories that will be immediately useful in future conversations."
            )
        }))
}

/// Get the prompts directory path from environment or use default
fn get_prompts_dir() -> std::path::PathBuf {
    if let Ok(prompts_dir) = std::env::var("PROMPTS_DIR") {
        std::path::PathBuf::from(prompts_dir)
    } else {
        std::path::PathBuf::from("prompts")
    }
}

// Global LRU cache for token counting (max 500 entries)
lazy_static! {
    static ref TOKEN_CACHE: Mutex<LruCache<[u8; 32], usize>> = {
        Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap()))
    };
}

/// Hash text to use as cache key (saves memory for large texts)
fn hash_text(text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Optimize memories using AI
async fn optimize_memories(
    transcript: &str,
    existing_memory: &str,
    context_window: usize,
) -> Result<MemoryOptimization> {
    // Validate inputs
    if transcript.is_empty() && existing_memory.is_empty() {
        anyhow::bail!("No content to optimize: both transcript and existing memory are empty");
    }
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Create AI client for GPT-5 calls
    let ai_client = UniversalAIClient::new(config.clone())?;
    
    // Load memory optimization prompt
    let prompt = load_memory_prompt().await?;
    
    // Calculate accurate token counts with global caching
    let prompt_tokens = count_tokens(&prompt)?;
    let existing_memory_tokens = count_tokens(&existing_memory)?;
    let transcript_tokens = count_tokens(&transcript)?;
    
    // Prepare context for AI - validate size
    let context = format!(
        "CONTEXT WINDOW LIMIT: {} tokens
        
EXISTING MEMORIES ({} tokens):
{}

NEW CONVERSATION TRANSCRIPT ({} tokens):
{}

Optimize and merge these memories, keeping only the most relevant information within the token limit.",
        context_window,
        existing_memory_tokens,
        if existing_memory.is_empty() { "No existing memories" } else { existing_memory },
        transcript_tokens,
        if transcript.is_empty() { "No new transcript" } else { transcript }
    );
    
    // Calculate exact total input tokens
    let context_tokens = count_tokens(&context)?;
    let total_input_tokens = prompt_tokens + context_tokens;
    
    if total_input_tokens > context_window {
        eprintln!("Warning: Input exceeds context window ({} tokens > {} limit)", total_input_tokens, context_window);
        // Truncate if necessary
        if total_input_tokens > context_window * 2 {
            return Err(anyhow::anyhow!("Input too large: {} tokens exceeds double the context window", total_input_tokens));
        }
    }
    
    // Use GPT-5-nano for memory optimization as per docs
    let model = "gpt-5-nano";
    
    // Call GPT-5 using UniversalAIClient
    let response = ai_client.optimize_memory_gpt5(&context, &prompt, model).await;
    
    match response {
        Ok(json_value) => {
            // Parse the JSON value into MemoryOptimization struct
            let optimization: MemoryOptimization = serde_json::from_value(json_value)
                .context("Failed to parse memory optimization response")?;
            Ok(optimization)
        },
        Err(e) => {
            // When AI is unavailable, don't do anything - just return error
            eprintln!("AI optimization failed: {}. Memory not optimized.", e);
            Err(e)
        }
    }
}

/// Count tokens in a string using global LRU cache
fn count_tokens(text: &str) -> Result<usize> {
    // Empty text shortcut
    if text.is_empty() {
        return Ok(0);
    }
    
    // Calculate hash for cache key
    let hash_key = hash_text(text);
    
    // Check cache first
    {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        if let Some(&count) = cache.get(&hash_key) {
            return Ok(count);
        }
    }
    
    // Count tokens using tiktoken
    let bpe = get_bpe_from_model("gpt-4")
        .context("Failed to initialize tokenizer")?;
    let count = bpe.encode_ordinary(text).len();
    
    // Store in LRU cache
    {
        let mut cache = TOKEN_CACHE.lock().unwrap();
        cache.put(hash_key, count);
    }
    
    Ok(count)
}

/// Format memories for storage
fn format_memories_for_storage(optimization: &MemoryOptimization) -> String {
    let mut output = String::new();
    
    // Add header
    output.push_str("# AI Conversation Memory\n\n");
    output.push_str(&format!("Last updated: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    output.push_str(&format!("Total tokens: {}\n", optimization.total_tokens));
    output.push_str(&format!("Reduction ratio: {:.1}%\n\n", optimization.reduction_ratio * 100.0));
    
    // Add key insights if any
    if !optimization.key_insights.is_empty() {
        output.push_str("## Key Insights\n");
        for insight in &optimization.key_insights {
            output.push_str(&format!("- {}\n", insight));
        }
        output.push_str("\n");
    }
    
    // Group memories by category
    let mut categories: std::collections::HashMap<String, Vec<&Memory>> = std::collections::HashMap::new();
    for memory in &optimization.optimized_memories {
        categories.entry(memory.category.clone()).or_insert_with(Vec::new).push(memory);
    }
    
    // Sort categories and format
    let mut sorted_categories: Vec<_> = categories.iter().collect();
    sorted_categories.sort_by_key(|&(category, _)| category);
    
    for (category, memories) in sorted_categories {
        output.push_str(&format!("## {}\n\n", category));
        
        // Sort memories by relevance score (highest first)
        let mut sorted_memories = memories.clone();
        sorted_memories.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        for memory in sorted_memories {
            output.push_str(&format!("**[{}] (relevance: {:.1}):**\n", memory.timestamp, memory.relevance_score));
            output.push_str(&format!("{}\n\n", memory.content));
        }
    }
    
    output
}


#[tokio::main]
async fn main() -> Result<()> {
    // Read hook input from stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).context("Failed to read stdin")?;
    
    // Parse the input
    let hook_input: MemoryHookInput = serde_json::from_str(&buffer)
        .context("Failed to parse input JSON")?;
    
    // Only process on stop event
    if hook_input.hook_event != "stop" {
        // Pass through - not a stop event
        let output = MemoryHookOutput {
            hook_specific_output: MemoryHookSpecificOutput {
                hook_event_name: "MemoryOrganizer".to_string(),
                memory_optimized: false,
                tokens_reduced: None,
                summary: Some("Skipped - not a stop event".to_string()),
            },
        };
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    }
    
    // Get paths
    let transcript_path = match hook_input.transcript_path {
        Some(path) => path,
        None => {
            eprintln!("Warning: No transcript path provided");
            let output = MemoryHookOutput {
                hook_specific_output: MemoryHookSpecificOutput {
                    hook_event_name: "MemoryOrganizer".to_string(),
                    memory_optimized: false,
                    tokens_reduced: None,
                    summary: Some("No transcript path provided".to_string()),
                },
            };
            println!("{}", serde_json::to_string(&output)?);
            return Ok(());
        }
    };
    
    let memory_path = hook_input.memory_path
        .unwrap_or_else(|| "CLAUDE.md".to_string());
    
    let context_window = hook_input.context_window.unwrap_or(8000);
    
    // Read transcript and existing memory
    let transcript = read_transcript(&transcript_path).await?;
    let existing_memory = read_memory(&memory_path).await?;
    
    // Optimize memories
    match optimize_memories(&transcript, &existing_memory, context_window).await {
        Ok(optimization) => {
            // Format and save optimized memories
            let formatted_memories = format_memories_for_storage(&optimization);
            write_memory(&memory_path, &formatted_memories).await?;
            
            // Calculate tokens reduced using tiktoken
            let original_tokens = count_tokens(&transcript)? + count_tokens(&existing_memory)?;
            let tokens_reduced = original_tokens.saturating_sub(optimization.total_tokens);
            
            // Return success response
            let output = MemoryHookOutput {
                hook_specific_output: MemoryHookSpecificOutput {
                    hook_event_name: "MemoryOrganizer".to_string(),
                    memory_optimized: true,
                    tokens_reduced: Some(tokens_reduced),
                    summary: Some(format!(
                        "Successfully optimized {} memories, reduced by {:.0}%, saved {} tokens",
                        optimization.optimized_memories.len(),
                        optimization.reduction_ratio * 100.0,
                        tokens_reduced
                    )),
                },
            };
            println!("{}", serde_json::to_string(&output)?);
        }
        Err(e) => {
            eprintln!("Memory optimization error: {}", e);
            
            // Return error response but don't block
            let output = MemoryHookOutput {
                hook_specific_output: MemoryHookSpecificOutput {
                    hook_event_name: "MemoryOrganizer".to_string(),
                    memory_optimized: false,
                    tokens_reduced: None,
                    summary: Some(format!("Optimization failed: {}", e)),
                },
            };
            println!("{}", serde_json::to_string(&output)?);
        }
    }
    
    Ok(())
}