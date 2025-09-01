# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a high-performance validation hooks system for Claude Code implemented in Rust. The project provides security and code quality validation through PreToolUse and PostToolUse hooks that integrate with AI models (OpenAI GPT-5 and xAI Grok) for intelligent validation.

## Architecture

### Core Components

**Binary Hooks** (src/bin/)
- `pretooluse.rs`: Pre-execution validation hook that validates tool calls before execution
  - Integrates with xAI Grok for security validation  
  - Performs project structure scanning and analysis
  - Enforces security policies and project conventions
- `posttooluse.rs`: Post-execution validation hook for code quality and test coverage
  - Uses GPT-5 for comprehensive code review
  - Validates edits, writes, and multi-edits
  - Performs test coverage analysis and quality checks

**Core Library** (src/)
- `lib.rs`: Common data structures and utilities for hooks
  - Defines HookInput/Output structures matching Claude Code's contract
  - Provides serialization/deserialization for hook communication
- `project_context.rs`: Project structure analysis and context gathering
  - Scans and analyzes project files
  - Builds comprehensive project context for AI validation
  - Handles file filtering and content extraction

**Validation Prompts** (prompts/)
- `edit_validation.txt`: Project structure and security validation rules
- `post_edit_validation.txt`: Code quality validation criteria

### Hook Communication Flow

1. Claude Code sends JSON input via stdin to hook binary
2. Hook processes input, calls AI model for validation if needed
3. Hook returns JSON response with decision (allow/ask/deny/block)
4. Response includes structured feedback for code improvements

## Development Commands

### Building
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build --bin pretooluse
cargo build --bin posttooluse
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in specific file
cargo test --test test_project_context_integration

# Run with verbose output
cargo test -v
```

### Running Hooks Locally
```bash
# Test pretooluse hook
echo '{"tool_name":"Edit","tool_input":{"file_path":"test.js"}}' | cargo run --bin pretooluse

# Test posttooluse hook  
echo '{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs"},"tool_output":"Success"}' | cargo run --bin posttooluse

# Run release versions
cargo run --release --bin pretooluse
cargo run --release --bin posttooluse
```

### Development Workflow
```bash
# Watch for changes and rebuild
cargo watch -x build

# Format code
cargo fmt

# Run linter
cargo clippy

# Check for issues without building
cargo check
```

## Hook Integration

### Environment Variables Required
- `XAI_API_KEY`: xAI API key for Grok integration
- `XAI_BASE_URL`: xAI API base URL (default: https://api.x.ai/v1)
- `OPENAI_API_KEY`: OpenAI API key for GPT-5
- `PRETOOL_MODEL`: Model for pretool validation (default: grok-code-fast-1)
- `POSTTOOL_MODEL`: Model for posttool validation (default: gpt-5)
- `PRETOOL_TIMEOUT`: Timeout for pretool hook in ms (default: 30000)
- `POSTTOOL_TIMEOUT`: Timeout for posttool hook in ms (default: 45000)

### Hook Configuration in Claude Code
Hooks are configured in Claude Code settings to intercept tool calls. The binaries must be built and accessible in the system PATH or specified with full paths.

## Key Implementation Details

### Security Validation (PreToolUse)
- Validates file operations against project structure conventions
- Detects security risks: code injection, secret exposure, dangerous operations
- Uses AI to analyze code patterns and potential vulnerabilities
- Returns structured decisions with risk assessments

### Code Quality Validation (PostToolUse)  
- Analyzes code changes for quality issues
- Validates test coverage and documentation
- Checks for anti-patterns and code smells
- Provides structured feedback with specific line-level issues

### Project Context Analysis
The `project_context` module builds comprehensive project understanding by:
- Scanning file structure and identifying key components
- Extracting relevant code snippets and patterns
- Building dependency graphs and architecture understanding
- Providing context to AI models for better validation

### Error Handling
- All hooks use structured JSON responses per Claude Code specification
- Errors are logged to stderr, never mixed with stdout JSON
- Timeouts and API failures are handled gracefully with fallback decisions
- Exit codes follow Claude Code conventions (0=success, 2=block)

## Testing Strategy

### Unit Tests
Located in `tests/unit/` - test individual components and functions

### Integration Tests  
Located in `tests/integration/` - test full hook flow with mocked AI responses

### Test Fixtures
Located in `tests/fixtures/` - sample project structures for testing

### Running Specific Test Categories
```bash
cargo test --test '*unit*'
cargo test --test '*integration*'
```

## Performance Considerations

- Release builds use LTO and single codegen unit for optimization
- AI calls have configurable timeouts to prevent hanging
- Project scanning is optimized with parallel processing where possible
- JSON parsing/serialization uses serde for efficiency

## Debugging

### Enable Debug Output
```bash
DEBUG_HOOKS=true cargo run --bin pretooluse
```

### View Hook Logs
Hooks log to stderr which can be captured:
```bash
cargo run --bin pretooluse 2>hook.log
```

### Test with Sample Input
Create test JSON files in `tests/fixtures/` for repeatable testing