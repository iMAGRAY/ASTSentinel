# ValidationCodeHook

High-performance validation hooks for Claude Code, providing real-time security and code quality analysis through AI-powered validation.

## Features

- 🛡️ **Security Validation**: Detects SQL injection, code injection, exposed secrets, and other vulnerabilities
- ✨ **Code Quality Analysis**: Validates code style, test coverage, and maintainability
- 🤖 **Multi-Provider AI Support**: Works with OpenAI (GPT-4/5), xAI (Grok), Anthropic (Claude), Google (Gemini)
- 🌍 **Multi-Language Support**: Automatically responds in user's language (Russian, English, etc.)
- ⚡ **High Performance**: Built in Rust for minimal overhead
- 📊 **Project Context Awareness**: Analyzes entire project structure for better validation

## Quick Start

### 1. Clone and Build

```bash
git clone https://github.com/yourusername/ValidationCodeHook.git
cd ValidationCodeHook
cargo build --release
```

### 2. Setup API Keys

**IMPORTANT:** The hooks WILL NOT WORK without real API keys!

```bash
# For development: use .env.local (gitignored)
cp hooks/.env.example hooks/.env.local
nano hooks/.env.local  # Add your REAL API keys here

# OR for production: use .env
cp hooks/.env.example hooks/.env
nano hooks/.env  # Add your REAL API keys here
```

See [SETUP_API_KEYS.md](SETUP_API_KEYS.md) for detailed instructions on getting API keys.

### 3. Install Hooks

Copy the compiled binaries to your hooks directory:

```bash
cp target/release/pretooluse.exe hooks/
cp target/release/posttooluse.exe hooks/
```

### 4. Configure Claude Code

Add to your Claude Code settings to use the validation hooks.

## Configuration

### Environment Variables

- `PRETOOL_PROVIDER`: AI provider for pre-validation (openai/xai/anthropic/google)
- `POSTTOOL_PROVIDER`: AI provider for post-validation
- `PRETOOL_MODEL`: Model to use (e.g., gpt-4, grok-code-fast-1)
- `POSTTOOL_MODEL`: Model for post-validation
- `OPENAI_API_KEY`: Your OpenAI API key
- `XAI_API_KEY`: Your xAI API key

See `.env.example` for all configuration options.

### Prompt Customization

Edit prompts in the `prompts/` directory:
- `edit_validation.txt`: Pre-execution validation rules
- `post_edit_validation.txt`: Post-execution quality checks
- `language_instruction.txt`: Language detection rules
- `json_template.txt`: Response format template

## Architecture

```
ValidationCodeHook/
├── src/
│   ├── bin/
│   │   ├── pretooluse.rs    # Pre-execution validation
│   │   └── posttooluse.rs   # Post-execution validation
│   ├── analysis/             # Project structure analysis
│   ├── providers/            # AI provider integrations
│   └── validation/           # Validation logic
├── hooks/                    # Production binaries
│   ├── pretooluse.exe
│   ├── posttooluse.exe
│   └── prompts/             # Production prompts
└── prompts/                  # Development prompts
```

## Development

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

### Syncing Prompts

```bash
./sync_prompts.sh
```

## Security Best Practices

1. **Never commit API keys** - Use `.env.local` for local development
2. **Rotate keys regularly** - Update API keys periodically
3. **Use restricted keys** - Create keys with minimal required permissions
4. **Review hook feedback** - Pay attention to security warnings

## Troubleshooting

### "API key not found"
- Check that `.env` file exists and contains valid keys
- Verify environment variables are set correctly

### "Model does not exist"
- Ensure you're using a valid model name (e.g., `gpt-4` instead of `gpt-4-turbo`)
- Check API access permissions for the model

### Hook not triggering
- Verify binaries are in the correct location
- Check Claude Code settings for hook configuration
- Review stderr output for error messages

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Built for [Claude Code](https://claude.ai/code) by Anthropic
- Powered by various AI providers (OpenAI, xAI, Anthropic, Google)
- Written in Rust for performance and reliability