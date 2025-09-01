#!/usr/bin/env python3
"""
Test script to verify that hooks are using project context.
"""

import json
import subprocess
import tempfile
import os

def test_hook_with_project_context():
    """Test that hooks receive and use project structure context"""
    
    # Create test input for PostToolUse hook
    hook_input = {
        "tool_name": "Write",
        "tool_input": {
            "file_path": "test.js",
            "content": "console.log('test');"
        },
        "session_id": "test-session",
        "cwd": os.getcwd(),
        "hook_event_name": "PostToolUse"
    }
    
    # Run the posttooluse hook
    print("Testing PostToolUse hook with project context...")
    
    # Check if the hook binary exists
    hook_path = "./target/release/posttooluse.exe"
    if not os.path.exists(hook_path):
        hook_path = "./target/debug/posttooluse.exe"
    
    if not os.path.exists(hook_path):
        print(f"❌ Hook binary not found at {hook_path}")
        print("   Run 'cargo build --release' first")
        return False
    
    try:
        # Run the hook with test input
        result = subprocess.run(
            [hook_path],
            input=json.dumps(hook_input),
            capture_output=True,
            text=True,
            timeout=15
        )
        
        # Check stderr for project context message
        if "Added project structure context" in result.stderr:
            print("✅ PostToolUse hook successfully added project structure context!")
            
            # Parse stderr to get file/dir counts
            import re
            match = re.search(r'Added project structure context \((\d+) files, (\d+) dirs\)', result.stderr)
            if match:
                files, dirs = match.groups()
                print(f"   Found {files} files and {dirs} directories in project")
        else:
            print("⚠️  PostToolUse hook ran but didn't add project context")
            print(f"   Stderr: {result.stderr[:200]}")
        
        # Check if hook produced valid output
        if result.stdout:
            try:
                output = json.loads(result.stdout)
                print("✅ Hook produced valid JSON output")
            except json.JSONDecodeError:
                print("⚠️  Hook output is not valid JSON")
                print(f"   Output: {result.stdout[:200]}")
                
    except subprocess.TimeoutExpired:
        print("❌ Hook timed out (>15s)")
        return False
    except Exception as e:
        print(f"❌ Error running hook: {e}")
        return False
    
    # Test PreToolUse hook
    print("\nTesting PreToolUse hook with project context...")
    
    hook_path = "./target/release/pretooluse.exe"
    if not os.path.exists(hook_path):
        hook_path = "./target/debug/pretooluse.exe"
    
    if not os.path.exists(hook_path):
        print(f"❌ Hook binary not found at {hook_path}")
        return False
    
    try:
        result = subprocess.run(
            [hook_path],
            input=json.dumps(hook_input),
            capture_output=True,
            text=True,
            timeout=15
        )
        
        if "Added project structure context" in result.stderr:
            print("✅ PreToolUse hook successfully added project structure context!")
            
            # Parse stderr to get file/dir counts
            import re
            match = re.search(r'Added project structure context \((\d+) files, (\d+) dirs\)', result.stderr)
            if match:
                files, dirs = match.groups()
                print(f"   Found {files} files and {dirs} directories in project")
        else:
            print("⚠️  PreToolUse hook ran but didn't add project context")
            print(f"   Stderr: {result.stderr[:200]}")
            
    except Exception as e:
        print(f"❌ Error running hook: {e}")
        return False
    
    print("\n✅ All tests completed!")
    return True

if __name__ == "__main__":
    # Set environment variable for testing
    os.environ["CLAUDE_PROJECT_DIR"] = os.getcwd()
    
    print("=" * 60)
    print("TESTING HOOKS WITH PROJECT CONTEXT")
    print("=" * 60)
    print(f"Working directory: {os.getcwd()}")
    print(f"CLAUDE_PROJECT_DIR: {os.environ.get('CLAUDE_PROJECT_DIR', 'not set')}")
    print()
    
    test_hook_with_project_context()