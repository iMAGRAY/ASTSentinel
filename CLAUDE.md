{
  "validation_hooks_system": {
    "project_name": "High-performance validation hooks for Claude Code",
    "description": "AI-driven security & quality enforcement via PreToolUse (xAI Grok) + PostToolUse (GPT-5)",
    "language": "Rust",
    "architecture": {
      "binaries": {
        "pretooluse": {
          "location": "src/bin/pretooluse.rs",
          "purpose": "Pre-execution validation hook - validates tool calls before execution",
          "ai_integration": "xAI Grok (grok-code-fast-1)",
          "functions": [
            "Integrates with xAI Grok for security validation",
            "Performs project structure scanning and analysis", 
            "Enforces security policies and project conventions"
          ]
        },
        "posttooluse": {
          "location": "src/bin/posttooluse.rs", 
          "purpose": "Post-execution validation hook for code quality and test coverage",
          "ai_integration": "GPT-5",
          "functions": [
            "Uses GPT-5 for comprehensive code review",
            "Validates edits, writes, and multi-edits",
            "Performs test coverage analysis and quality checks"
          ]
        }
      },
      "core_library": {
        "lib.rs": {
          "purpose": "Common data structures and utilities for hooks",
          "features": [
            "Defines HookInput/Output structures matching Claude Code's contract",
            "Provides serialization/deserialization for hook communication"
          ]
        },
        "project_context.rs": {
          "purpose": "Project structure analysis and context gathering",
          "features": [
            "Scans and analyzes project files",
            "Builds comprehensive project context for AI validation",
            "Handles file filtering and content extraction"
          ]
        }
      },
      "validation_prompts": {
        "edit_validation.txt": "Project structure and security validation rules",
        "post_edit_validation.txt": "Code quality validation criteria"
      }
    },
    "hook_communication_flow": [
      "Claude Code sends JSON input via stdin to hook binary",
      "Hook processes input, calls AI model for validation if needed",
      "Hook returns JSON response with decision (allow/ask/deny/block)",
      "Response includes structured feedback for code improvements"
    ],
    "development_commands": {
      "building": {
        "debug": "cargo build",
        "release": "cargo build --release", 
        "specific_binary": "cargo build --bin pretooluse|posttooluse"
      },
      "testing": {
        "all": "cargo test",
        "with_output": "cargo test -- --nocapture",
        "specific": "cargo test test_name",
        "specific_file": "cargo test --test test_project_context_integration",
        "verbose": "cargo test -v",
        "categories": {
          "unit": "cargo test --test '*unit*'",
          "integration": "cargo test --test '*integration*'"
        }
      },
      "running_hooks": {
        "pretooluse_test": "echo '{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"test.js\"}}' | cargo run --bin pretooluse",
        "posttooluse_test": "echo '{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"src/main.rs\"},\"tool_output\":\"Success\"}' | cargo run --bin posttooluse",
        "release": "cargo run --release --bin pretooluse|posttooluse"
      },
      "development_workflow": {
        "watch": "cargo watch -x build",
        "format": "cargo fmt",
        "lint": "cargo clippy",
        "check": "cargo check"
      }
    },
    "environment_variables": [
      "XAI_API_KEY - xAI API key for Grok integration",
      "XAI_BASE_URL - xAI API base URL (default: https://api.x.ai/v1)",
      "OPENAI_API_KEY - OpenAI API key for GPT-5",
      "PRETOOL_MODEL - Model for pretool validation (default: grok-code-fast-1)",
      "POSTTOOL_MODEL - Model for posttool validation (default: gpt-5)", 
      "PRETOOL_TIMEOUT - Timeout for pretool hook in ms (default: 30000)",
      "POSTTOOL_TIMEOUT - Timeout for posttool hook in ms (default: 45000)"
    ],
    "key_implementation_details": {
      "security_validation": [
        "Validates file operations against project structure conventions",
        "Detects security risks: code injection, secret exposure, dangerous operations",
        "Uses AI to analyze code patterns and potential vulnerabilities",
        "Returns structured decisions with risk assessments"
      ],
      "code_quality_validation": [
        "Analyzes code changes for quality issues",
        "Validates test coverage and documentation", 
        "Checks for anti-patterns and code smells",
        "Provides structured feedback with specific line-level issues"
      ],
      "project_context_analysis": [
        "Scanning file structure and identifying key components",
        "Extracting relevant code snippets and patterns",
        "Building dependency graphs and architecture understanding",
        "Providing context to AI models for better validation"
      ],
      "error_handling": [
        "All hooks use structured JSON responses per Claude Code specification",
        "Errors are logged to stderr, never mixed with stdout JSON",
        "Timeouts and API failures are handled gracefully with fallback decisions",
        "Exit codes follow Claude Code conventions (0=success, 2=block)"
      ]
    },
    "testing_strategy": {
      "unit_tests": "tests/unit/ - test individual components and functions",
      "integration_tests": "tests/integration/ - test full hook flow with mocked AI responses",
      "test_fixtures": "tests/fixtures/ - sample project structures for testing"
    },
    "performance_considerations": [
      "Release builds use LTO and single codegen unit for optimization",
      "AI calls have configurable timeouts to prevent hanging",
      "Project scanning is optimized with parallel processing where possible", 
      "JSON parsing/serialization uses serde for efficiency"
    ],
    "debugging": {
      "enable_debug": "DEBUG_HOOKS=true cargo run --bin pretooluse",
      "view_logs": "cargo run --bin pretooluse 2>hook.log",
      "sample_input": "Create test JSON files in tests/fixtures/ for repeatable testing"
    }
  },
  "gpt5_api_guide": {
    "critical_differences": {
      "model_type": "GPT-5 is a reasoning model with cardinal parameter limitations",
      "unsupported_parameters_reasoning_models": [
        "temperature - MUST BE 1 or omitted entirely",
        "top_p - NOT SUPPORTED", 
        "logprobs - NOT SUPPORTED",
        "top_logprobs - NOT SUPPORTED",
        "logit_bias - NOT SUPPORTED",
        "stop - NOT SUPPORTED (only in Chat Completions API)"
      ],
      "common_errors": {
        "temperature_error": {
          "message": "Unsupported parameter: 'temperature' is not supported with this model.",
          "type": "invalid_request_error"
        },
        "top_p_error": {
          "message": "Unsupported parameter: 'top_p' is not supported with this model.",
          "type": "invalid_request_error"
        }
      }
    },
    "model_lineup": {
      "gpt-5": {
        "context": 400000,
        "output": 128000,
        "price_input_per_1m": 1.25,
        "price_output_per_1m": 10.00,
        "reasoning": true,
        "temperature": "Only 1"
      },
      "gpt-5-mini": {
        "context": 400000,
        "output": 128000, 
        "price_input_per_1m": 0.25,
        "price_output_per_1m": 2.00,
        "reasoning": true,
        "temperature": "Only 1"
      },
      "gpt-5-nano": {
        "context": 400000,
        "output": 128000,
        "price_input_per_1m": 0.05,
        "price_output_per_1m": 0.40,
        "reasoning": true,
        "temperature": "Only 1"
      },
      "gpt-5-chat": {
        "context": 128000,
        "output": 16384,
        "price_input_per_1m": 1.25,
        "price_output_per_1m": 10.00,
        "reasoning": false,
        "temperature": "Supported 0.0-2.0"
      }
    },
    "knowledge_cutoff": "May 31, 2024",
    "key_model_differences": {
      "reasoning_models": [
        "Always use internal 'thinking' (reasoning tokens)",
        "Limited parameter set",
        "Better for complex tasks, analysis, code",
        "Slower but higher quality"
      ],
      "gpt_5_chat": [
        "Supports traditional parameters (temperature, top_p)",
        "Does NOT support reasoning_effort",
        "Faster for simple dialogues",
        "Optimized for chatbots"
      ]
    },
    "supported_parameters": {
      "reasoning_models": {
        "chat_completions_api": {
          "supported": ["model", "messages", "max_tokens", "n", "stream", "user", "reasoning_effort", "tools", "tool_choice", "response_format"],
          "not_supported": ["input", "max_output_tokens", "temperature", "top_p", "stop", "presence_penalty", "frequency_penalty", "logit_bias", "logprobs", "verbosity"]
        },
        "responses_api": {
          "supported": ["model", "input", "max_output_tokens", "stream", "user", "reasoning.effort", "text.verbosity", "tools", "tool_choice", "text.format"],
          "not_supported": ["messages", "max_tokens", "temperature", "top_p", "n", "stop", "presence_penalty", "frequency_penalty", "logit_bias", "logprobs"]
        }
      },
      "gpt_5_chat": {
        "supported": ["temperature", "top_p"],
        "not_supported": ["reasoning_effort", "verbosity"]
      }
    },
    "correct_api_examples": {
      "chat_completions_reasoning": {
        "model": "gpt-5",
        "messages": [{"role": "user", "content": "Explain quantum computers"}],
        "max_tokens": 1000,
        "reasoning_effort": "medium"
      },
      "chat_completions_gpt5_chat": {
        "model": "gpt-5-chat",
        "messages": [{"role": "user", "content": "Hello"}],
        "temperature": 0.7,
        "top_p": 0.9,
        "max_tokens": 1000
      },
      "responses_api_basic": {
        "model": "gpt-5",
        "input": "Analyze algorithm efficiency",
        "max_output_tokens": 2000,
        "reasoning": {"effort": "high"},
        "text": {"verbosity": "medium"},
        "tools": [{"type": "function", "function": {}}]
      },
      "responses_api_structured": {
        "model": "gpt-5-nano",
        "input": "Extract names, places and dates from text",
        "reasoning": {"effort": "minimal"},
        "text": {
          "format": {
            "type": "json_schema",
            "json_schema": {
              "name": "entity_extraction",
              "schema": {
                "type": "object",
                "properties": {
                  "names": {"type": "array", "items": {"type": "string"}},
                  "locations": {"type": "array", "items": {"type": "string"}},
                  "dates": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["names", "locations", "dates"],
                "additionalProperties": false
              },
              "strict": true
            }
          }
        }
      }
    },
    "apis": {
      "chat_completions": {
        "endpoint": "POST https://api.openai.com/v1/chat/completions",
        "description": "Traditional chat completions API"
      },
      "responses_preferred": {
        "endpoint": "POST https://api.openai.com/v1/responses", 
        "description": "Recommended API for GPT-5 for maximum performance"
      }
    },
    "new_parameters": {
      "reasoning_effort": {
        "description": "Controls depth of model 'thinking' before response",
        "levels": {
          "minimal": {
            "reasoning_tokens": "0-50",
            "response_time": "Fast",
            "quality": "Basic",
            "cost": "Low",
            "use_cases": ["Data extraction", "formatting"]
          },
          "low": {
            "reasoning_tokens": "50-200", 
            "response_time": "Fast",
            "quality": "Good", 
            "cost": "Low",
            "use_cases": ["FAQ", "simple queries"]
          },
          "medium": {
            "reasoning_tokens": "200-1000",
            "response_time": "Medium",
            "quality": "Very good",
            "cost": "Medium", 
            "use_cases": ["Default", "content creation"]
          },
          "high": {
            "reasoning_tokens": "1000+",
            "response_time": "Slow",
            "quality": "Excellent",
            "cost": "High",
            "use_cases": ["Research", "complex analysis"]
          }
        },
        "chat_completions_syntax": "reasoning_effort: 'high'",
        "responses_api_syntax": "reasoning: {'effort': 'high'}"
      },
      "verbosity": {
        "description": "Controls response length and detail (Responses API only)",
        "levels": {
          "low": "Brief, compressed responses",
          "medium": "Balanced responses (default)",
          "high": "Detailed, comprehensive responses"
        },
        "syntax": "text: {'verbosity': 'low'}"
      }
    },
    "migration_guide": {
      "temperature_to_reasoning_effort": {
        "temp_0.3_or_less": "reasoning_effort: 'minimal'",
        "temp_0.3_to_0.7": "reasoning_effort: 'low'", 
        "temp_0.7_to_1.0": "reasoning_effort: 'medium'",
        "temp_above_1.0": "reasoning_effort: 'high'"
      },
      "common_migration_errors": [
        "Using old parameters with GPT-5",
        "Not accounting for reasoning tokens in cost calculation",
        "Using Chat Completions API instead of Responses API"
      ],
      "cost_calculation_change": "Must account for reasoning_tokens: reasoning_tokens = response.usage.completion_tokens_details.reasoning_tokens"
    },
    "rate_limits": {
      "tier_1": {"rpm": 500, "tpm": 200000, "batch_queue": "2M"},
      "tier_2": {"rpm": 5000, "tpm": 2000000, "batch_queue": "20M"},
      "tier_3": {"rpm": 5000, "tpm": 4000000, "batch_queue": "40M"},
      "tier_4": {"rpm": 10000, "tpm": 10000000, "batch_queue": "1B"},
      "tier_5": {"rpm": 30000, "tpm": 180000000, "batch_queue": "15B"}
    },
    "use_case_examples": {
      "simple_chatbot": "Use gpt-5-chat with temperature",
      "complex_analysis": "Use gpt-5 with Responses API",
      "data_extraction": "Use gpt-5-nano with minimal reasoning",
      "structured_output": "Use text.format with json_schema"
    }
  },
  "claude_code_sdk": {
    "overview": {
      "description": "Programmatic access to Claude Code agent architecture",
      "key_features": [
        "Optimized Claude integration with automatic prompt caching",
        "Rich tool ecosystem: file operations, code execution, web search, MCP extensibility",
        "Advanced permissions with precise agent capability control",
        "Production capabilities: built-in error handling, session management, monitoring"
      ],
      "available_forms": [
        "Headless Mode - CLI scripts and automation",
        "TypeScript SDK - Node.js and web applications", 
        "Python SDK - Python applications and data science"
      ]
    },
    "installation": {
      "requirements": {
        "python": "Python 3.10+",
        "nodejs": "Node.js 18+ (for all variants)"
      },
      "typescript_sdk": "npm install -g @anthropic-ai/claude-code",
      "python_sdk": [
        "pip install claude-code-sdk",
        "npm install -g @anthropic-ai/claude-code  # Required dependency"
      ]
    },
    "python_sdk": {
      "main_interface": "ClaudeSDKClient",
      "simple_interface": "query function",
      "critical_options": {
        "system_prompt": "Custom system instructions",
        "append_system_prompt": "Additional system instructions",
        "max_turns": "Conversation length limit",
        "model": "claude-3-5-sonnet-20241022",
        "max_thinking_tokens": 8000,
        "allowed_tools": ["Bash", "Read", "Write"],
        "disallowed_tools": ["WebSearch"],
        "continue_conversation": false,
        "resume": "session-uuid",
        "cwd": "/path/to/working/directory",
        "add_dirs": ["/additional/context/dir"],
        "settings": "/path/to/settings.json",
        "permission_mode": ["default", "acceptEdits", "plan", "bypassPermissions"],
        "permission_prompt_tool_name": "mcp__approval_tool",
        "mcp_servers": "Custom MCP server configurations",
        "extra_args": "Additional CLI arguments"
      },
      "permission_modes": {
        "default": "CLI prompts for dangerous tools (default behavior)",
        "acceptEdits": "Automatically accept file edits without prompt",
        "plan": "Planning mode - analyze without making changes",
        "bypassPermissions": "Allow all tools without prompt (use carefully)"
      }
    },
    "typescript_sdk": {
      "basic_usage": "query function with options",
      "configuration_parameters": {
        "abortController": "AbortController for operation cancellation",
        "additionalDirectories": "Additional directories to include",
        "allowedTools": "List of permitted tools",
        "appendSystemPrompt": "Text to add to system prompt",
        "canUseTool": "Custom permission function", 
        "continue": "Continue last session",
        "customSystemPrompt": "Complete system prompt replacement",
        "cwd": "Working directory",
        "disallowedTools": "List of forbidden tools",
        "maxTurns": "Maximum number of turns",
        "permissionMode": "Permission mode"
      },
      "streaming_with_images": "Supports base64 image input in messages",
      "custom_tools": "MCP servers with createSdkMcpServer"
    },
    "headless_mode": {
      "basic": "claude -p 'query'",
      "with_config": "claude -p 'query' --allowed-tools 'Read,Write,Bash' --max-turns 3 --output-format json"
    }
  },
  "claude_code_hooks": {
    "description": "Custom shell commands executed automatically at specific Claude Code lifecycle points",
    "hook_events": [
      "PreToolUse - before tool usage",
      "PostToolUse - after successful tool execution", 
      "Notification - when Claude sends notifications",
      "UserPromptSubmit - when user submits prompt",
      "Stop - when Claude response ends",
      "SubagentStop - when subagent work ends", 
      "PreCompact - before compacting",
      "SessionStart - at session start",
      "SessionEnd - at session end"
    ],
    "configuration_files": [
      "~/.claude/settings.json - user settings",
      ".claude/settings.json - project settings", 
      ".claude/settings.local.json - local project settings"
    ],
    "configuration_structure": {
      "matcher": "Tool pattern for PreToolUse/PostToolUse (regex supported, * for all)",
      "hooks": "Array of commands to execute",
      "type": "Currently only 'command' supported",
      "command": "Bash command to execute",
      "timeout": "Optional timeout in seconds"
    },
    "input_output": {
      "input_format": "JSON via stdin with session_id, transcript_path, cwd, hook_event_name, tool_name, tool_input, tool_response",
      "output_simple": {
        "exit_0": "Success - stdout shown to user",
        "exit_2": "Blocking error - stderr passed to Claude for automatic handling",
        "exit_other": "Non-blocking error - stderr shown to user"
      },
      "output_advanced": "JSON with continue, stopReason, suppressOutput, systemMessage, decision, reason, hookSpecificOutput"
    },
    "environment_variables": [
      "$CLAUDE_FILE_PATHS - affected file paths",
      "$CLAUDE_PROJECT_DIR - project directory"
    ],
    "example_hooks": {
      "bash_validation": "Python script that validates bash commands using regex patterns",
      "context_addition": "Add current time context on UserPromptSubmit",
      "code_formatting": "Auto-format code after Write/Edit operations"
    }
  },
  "configuration_and_settings": {
    "settings_hierarchy": [
      "Enterprise managed policies (highest priority)",
      "Command line arguments",
      "Local project settings (.claude/settings.local.json)", 
      "General project settings (.claude/settings.json)",
      "User settings (~/.claude/settings.json) (lowest priority)"
    ],
    "enterprise_locations": {
      "macos": "/Library/Application Support/ClaudeCode/managed-settings.json",
      "linux_wsl": "/etc/claude-code/managed-settings.json",
      "windows": "C:\\ProgramData\\ClaudeCode\\managed-settings.json"
    },
    "available_settings": {
      "apiKeyHelper": "Script for generating auth value",
      "cleanupPeriodDays": "Transcript retention period in days",
      "env": "Environment variables for each session",
      "permissions": "Tool permission settings",
      "hooks": "Hook configuration", 
      "model": "Default model",
      "forceLoginMethod": "Force login method (claudeai or console)"
    },
    "permission_settings": {
      "allow": "Array of tool permission rules",
      "ask": "Array of confirmation request rules",
      "deny": "Array of tool denial rules", 
      "additionalDirectories": "Additional working directories",
      "defaultMode": "Default permission mode"
    }
  },
  "cli_commands": {
    "basic_commands": {
      "interactive_repl": "claude",
      "repl_with_prompt": "claude 'query'",
      "sdk_then_exit": "claude -p 'query'",
      "process_pipe": "cat file | claude -p 'query'",
      "continue_last": "claude -c",
      "resume_session": "claude -r '<session-id>' 'query'",
      "update": "claude update"
    },
    "cli_flags": {
      "add_dir": "Add additional working directories",
      "allowedTools": "List of permitted tools",
      "disallowedTools": "List of forbidden tools", 
      "print": "Print response without interactive mode (-p)",
      "append_system_prompt": "Add to system prompt",
      "output_format": "Output format (text, json, stream-json)",
      "verbose": "Enable detailed logging",
      "max_turns": "Limit number of turns",
      "model": "Set model for session",
      "permission_mode": "Start in specified permission mode",
      "continue": "Load last dialogue"
    }
  },
  "custom_slash_commands": {
    "types": {
      "project_commands": {
        "location": ".claude/commands/",
        "sharing": "Via git with project"
      },
      "personal_commands": {
        "location": "~/.claude/commands/",
        "scope": "Available in all projects"
      }
    },
    "arguments": {
      "all_arguments": "$ARGUMENTS - all arguments as single string",
      "individual_arguments": "$1, $2, $3... - individual positional arguments"
    },
    "namespacing": "Organize in subdirectories for namespaced commands"
  },
  "xai_grok_code_fast_1": {
    "purpose": "Agent coding model (pair-programmer, tool calling, IDE/CLI agents)",
    "goal": "Maximum response speed with sufficient accuracy for practical development tasks", 
    "usage": "Patch generation, project navigation, tool calls (lint/test/build), structured responses",
    "model_id": "grok-code-fast-1",
    "properties": [
      "Long context support for large diffs/tool logs",
      "Output modes: plain text, streaming, structured JSON",
      "Reasoning support in stream (reasoning deltas)",
      "Optimized for short iterations and multi-step agent scenarios"
    ],
    "api_compatibility": {
      "base_host": "https://api.x.ai",
      "openai_compatible": "POST /v1/chat/completions",
      "anthropic_compatible": "POST /v1/messages",
      "additional_endpoints": [
        "/v1/models - model listing",
        "/v1/tokenize-text - cost/context planning"
      ]
    },
    "correct_api_examples": {
      "basic_curl": {
        "url": "https://api.x.ai/v1/chat/completions",
        "headers": ["Authorization: Bearer $XAI_API_KEY", "Content-Type: application/json"],
        "body": {
          "model": "grok-code-fast-1",
          "messages": [
            {"role": "system", "content": "You are a code assistant. Respond concisely, provide patches."},
            {"role": "user", "content": "Optimize sorting function for memory."}
          ],
          "temperature": 0.2,
          "stream": true
        }
      },
      "nodejs_openai_sdk": {
        "client_config": {
          "apiKey": "process.env.XAI_API_KEY",
          "baseURL": "https://api.x.ai/v1"
        },
        "request": {
          "model": "grok-code-fast-1", 
          "messages": [
            {"role": "system", "content": "Brief. Return unified-diff without preamble."},
            {"role": "user", "content": "Update parse() function for new format."}
          ],
          "stream": true
        }
      },
      "anthropic_compatible": {
        "url": "https://api.x.ai/v1/messages",
        "body": {
          "model": "grok-code-fast-1",
          "messages": [{"role": "user", "content": "Generate deploy.sh shell script with checks."}],
          "stream": true
        }
      },
      "tool_calling": {
        "model": "grok-code-fast-1",
        "messages": [{"role": "user", "content": "Generate DB migrations"}],
        "tools": [{
          "type": "function",
          "function": {
            "name": "apply_migration",
            "description": "Apply SQL migration safely", 
            "parameters": {
              "type": "object",
              "properties": {"sql": {"type": "string"}},
              "required": ["sql"]
            }
          }
        }],
        "tool_choice": "auto"
      },
      "structured_response": {
        "model": "grok-code-fast-1",
        "messages": [{"role": "user", "content": "Return JSON with function, files, risk fields"}],
        "response_format": {
          "type": "json_schema",
          "json_schema": {
            "name": "ActionPlan",
            "schema": {
              "type": "object",
              "properties": {
                "function": {"type": "string"},
                "files": {"type": "array", "items": {"type": "string"}},
                "risk": {"type": "string"}
              },
              "required": ["function", "files", "risk"],
              "additionalProperties": false
            }
          }
        }
      }
    },
    "streaming_and_reasoning": [
      "Always include stream: true for IDE/agent interactivity",
      "Stream may contain regular output deltas and reasoning deltas",
      "Show reasoning to user per product policy (can hide, log, trim PII)",
      "Aggregate deltas in order received"
    ],
    "tool_calling": {
      "format": "Compatible with function calling",
      "tool_description": "JSON schema in tools field",
      "control_options": {
        "tool_choice_auto": "Model decides whether to call tool",
        "tool_choice_required": "Forced call (possible argument guessing)",
        "tool_choice_specific": "Force specific function",
        "parallel_calls": "Enabled by default"
      }
    },
    "structured_responses": {
      "support": "Strict JSON per specified schema",
      "supported_types": ["string", "number", "integer", "float", "object", "array", "boolean", "enum", "anyOf"],
      "limitations": "minLength/maxLength, minItems/maxItems, allOf may be ignored",
      "recommendation": "Validate on client side (Zod/Pydantic/JSON Schema)"
    },
    "prompting_patterns": [
      "Provide exact context: file paths, dependency versions, edit goals",
      "State readiness criteria (compiles, tests pass, style rules)",
      "Request unified-diff for direct patch application",
      "Break tasks into short steps for better scoring and latency",
      "For deep investigations combine with more 'thinking' model and/or external search/tools"
    ],
    "performance_and_limits": [
      "Input cache reduces cost and speeds up step series - don't change common prefix unnecessarily",
      "Token-based billing (prompt/completion; reasoning when present)",
      "Handle 429 errors (backoff, jitter, idempotent retries)",
      "Monitor usage in response for budget planning"
    ],
    "ide_integration": [
      "Supported in popular agent IDEs and plugin environments",
      "BYOK: use own xAI API key and baseURL https://api.x.ai/v1",
      "Multi-routing possible via proxy providers"
    ],
    "diagnostics_and_resilience": [
      "Log: input, output, tool_calls, usage metadata, error codes", 
      "Retry strategies: exponential backoff with attempt limits",
      "Long tasks - use deferred completions/async patterns",
      "Incident monitoring - via provider status page"
    ],
    "usage_patterns": {
      "patch_generation": [
        "Give path and minimal fragment",
        "Request unified-diff without preamble", 
        "Apply via apply_patch tool"
      ],
      "agent_diagnostics": [
        "Goal in system prompt (temp/* branch, security rules)",
        "Tools: read_files, ripgrep, tests.run, apply_patch",
        "Enable parallel tool calls; stream reasoning in UI if needed"
      ],
      "structured_response": [
        "Set JSON schema (example: {function: string, files: string[], risk: string})",
        "Enable strict output; validate on client"
      ]
    },
    "limitations": [
      "No built-in Live Search - use separate search tool",
      "Reasoning trace available in stream; storage/display per privacy policy", 
      "For very deep code analysis better combine with more 'heavy' model"
    ]
  },
  "practical_examples": {
    "sre_incident_investigation": "Python async example using ClaudeSDKClient with MCP servers for monitoring",
    "automated_pr_security_audit": "TypeScript example using query function for PR diff analysis",
    "automatic_code_formatting_hook": "Hook configuration for prettier on Write/Edit operations",
    "api_scaffolding_slash_command": "Custom slash command template for REST API endpoint creation"
  }
}