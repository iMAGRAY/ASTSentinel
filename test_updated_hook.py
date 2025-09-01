#!/usr/bin/env python3
"""
Test file to verify updated PostToolUse hook format
Contains multiple security issues for testing
"""

import os
import subprocess

class VulnerableApp:
    def __init__(self):
        # Security flaw: hardcoded credentials
        self.db_password = "admin123"
        self.api_secret = "sk-test-abc123def456ghi789"
        
    def search_users(self, query):
        # SQL injection vulnerability 
        sql = f"SELECT * FROM users WHERE name LIKE '%{query}%'"
        return sql
        
    def execute_command(self, cmd):
        # Command injection risk
        os.system(f"echo Processing: {cmd}")
        
    def process_data(self, user_input):
        # Code injection via eval
        if "calculate" in user_input:
            result = eval(user_input.replace("calculate", ""))
            return result
            
    def backup_system(self, path):
        # Path traversal vulnerability
        subprocess.run(f"tar -czf backup.tar.gz {path}", shell=True)

if __name__ == "__main__":
    app = VulnerableApp()
    print("Testing hook format with security issues...")