import subprocess
import sys
import os

os.chdir(r"C:\Users\1\Documents\GitHub\ValidationCodeHook")

try:
    # Run git diff command
    result = subprocess.run(['git', 'diff', 'HEAD~1', 'src/bin/pretooluse.rs'], 
                          capture_output=True, text=True)
    
    if result.returncode == 0:
        lines = result.stdout.split('\n')[:50]  # First 50 lines
        for line in lines:
            print(line)
    else:
        print(f"Error: {result.stderr}")
        
except Exception as e:
    print(f"Exception: {e}")