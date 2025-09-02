#!/bin/bash

# Test script to verify diff formatting in hooks

echo "Testing Edit operation diff formatting..."

# Create test input for Edit operation
cat > test_edit_input.json << 'EOF'
{
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "tests/test_diff_hooks.js",
    "old_string": "    let total = 0;",
    "new_string": "    let total = 0; // Initialize sum"
  },
  "transcript_path": null
}
EOF

echo "Running posttooluse hook with Edit operation..."
cat test_edit_input.json | cargo run --bin posttooluse 2>/dev/null | jq -r '.hook_specific_output.additional_context' 2>/dev/null || echo "Test completed"

echo ""
echo "Testing Write operation diff formatting..."

# Create test input for Write operation
cat > test_write_input.json << 'EOF'
{
  "tool_name": "Write",
  "tool_input": {
    "file_path": "tests/new_test.js",
    "content": "// New test file\nfunction test() {\n    return 'hello';\n}"
  },
  "transcript_path": null
}
EOF

echo "Running posttooluse hook with Write operation..."
cat test_write_input.json | cargo run --bin posttooluse 2>/dev/null | jq -r '.hook_specific_output.additional_context' 2>/dev/null || echo "Test completed"

# Cleanup
rm -f test_edit_input.json test_write_input.json

echo ""
echo "Diff formatting test completed!"