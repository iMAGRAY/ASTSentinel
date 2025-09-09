# Test Data Directory

This directory contains test fixtures and sample files used for testing the validation hooks and AST analyzer.

## Important Security Notice

Files in this directory, particularly `test_problems.py` and `test_security_issues.py`, contain **intentional security vulnerabilities** for testing purposes:

- SQL injection vulnerabilities
- Hardcoded credentials
- Dead code
- High cyclomatic complexity
- Other code quality issues

**These files are NOT meant for production use and should NEVER be deployed or used as examples of good code.**

## Purpose

These test files are used to verify that the AST analyzer correctly identifies:
- Security vulnerabilities (SQL injection, hardcoded credentials)
- Code quality issues (dead code, complexity)
- Best practice violations

## Files

- `test_problems.py` - Python file with various intentional issues for AST testing
- `test_security_issues.py` - Python file focused on security vulnerabilities
- `json/` - JSON test fixtures for hook validation
- `rust/` - Rust test files for diff formatting and output testing
- `scripts/` - Shell script test fixtures

## Usage

These files are automatically used by the test suite. Do not modify them unless you're updating the test cases themselves.