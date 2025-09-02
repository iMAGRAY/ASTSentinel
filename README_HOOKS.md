# Validation Hooks for Claude Code

## Overview
This project provides security and code quality validation hooks for Claude Code.

## Installation
1. Build the hooks: `cargo build --release`
2. Copy to hooks directory: Already done!
3. Configure Claude Code to use the hooks

## Configuration
Edit `.env` file to configure:
- API providers (OpenAI, Anthropic, Google, xAI)
- Models for validation
- Timeout settings
- Debug options

## Hook Binaries
- `pretooluse.exe` - Pre-execution validation
- `posttooluse.exe` - Post-execution validation

Both hooks are located in the `hooks/` directory and are ready to use.