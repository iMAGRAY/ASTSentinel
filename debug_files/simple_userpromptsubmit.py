#!/usr/bin/env python3
import json
import sys
import os
import datetime

try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError:
    # If no JSON input, continue silently
    input_data = {}

# Add project context as plain text (not JSON)
context_parts = []

# Add current timestamp
context_parts.append(f"Current time: {datetime.datetime.now()}")

# Add basic project info
try:
    cwd = os.getcwd()
    context_parts.append(f"Working directory: {cwd}")
    
    # Count files
    file_count = 0
    for root, dirs, files in os.walk("."):
        # Skip hidden directories
        dirs[:] = [d for d in dirs if not d.startswith('.')]
        file_count += len(files)
    
    context_parts.append(f"Project has {file_count} files")
    
    # Check for common config files
    config_files = []
    for f in ['package.json', 'Cargo.toml', 'requirements.txt', 'pyproject.toml']:
        if os.path.exists(f):
            config_files.append(f)
    
    if config_files:
        context_parts.append(f"Config files: {', '.join(config_files)}")

except Exception:
    context_parts.append("Could not analyze project structure")

# Output as simple text (not JSON)
context = "\n".join(context_parts)
print(context)
sys.exit(0)