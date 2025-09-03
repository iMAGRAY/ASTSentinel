#!/bin/bash

# Simple test of full context diff formatting

echo "Creating test file..."
cat > test_sample.js << 'EOF'
function hello() {
    console.log("Hello");
    return true;
}

function goodbye() {
    console.log("Goodbye");
    return false;
}
EOF

echo "Creating Edit test input..."
cat > test_edit.json << 'EOF'
{
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "test_sample.js",
    "old_string": "console.log(\"Hello\");",
    "new_string": "console.log(\"Hello, World!\");"
  },
  "transcript_path": null
}
EOF

echo ""
echo "Testing Edit operation with FULL file context:"
echo "================================================"

# Set minimal env vars and run
export XAI_API_KEY="test-key"
export OPENAI_API_KEY="test-key"
export PRETOOL_PROVIDER="xai"
export POSTTOOL_PROVIDER="openai"
export DEBUG_HOOKS="true"

cat test_edit.json | cargo run --bin posttooluse 2>&1 | grep -A 50 "=== Full file"

# Cleanup
rm -f test_sample.js test_edit.json

echo ""
echo "Test completed!"