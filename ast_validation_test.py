# Test file to validate AST analysis rules
import os
import sys
import json

# Function with too many parameters (should trigger TooManyParameters - 8 > 5)
def function_with_excessive_params(param1, param2, param3, param4, param5, param6, param7, param8):
    """This function has 8 parameters, exceeding the limit of 5"""
    return param1 + param2 + param3 + param4 + param5 + param6 + param7 + param8

# Function with deep nesting (should trigger DeepNesting - 8 levels > 6)
def deeply_nested_logic():
    """Deep nesting test - 8 levels deep"""
    if True:  # Level 1
        if True:  # Level 2
            if True:  # Level 3
                if True:  # Level 4
                    if True:  # Level 5
                        if True:  # Level 6
                            if True:  # Level 7
                                if True:  # Level 8 - should trigger warning
                                    return "too deep"
    return "not reached"

# Long line test (should trigger LongLine - exceeds 120 characters)
def test_long_lines():
    # This line is intentionally very long to test the long line detection rule in the AST analyzer - it should definitely exceed 120 characters
    short_line = "ok"
    return short_line

# Security issues - hardcoded credentials (should trigger HardcodedCredentials)
def security_problems():
    api_key = "sk-1234567890abcdef1234567890abcdef1234567890"  # OpenAI-style key
    secret_token = "this_is_a_very_long_secret_that_should_be_detected_as_credential"
    password = "admin123password"
    return api_key, secret_token, password

# SQL injection risk (should trigger SqlInjection)
def database_query_risk(user_id):
    # F-string with SQL - dangerous pattern
    query = f"SELECT * FROM users WHERE id = {user_id} AND status = 'active'"
    return query

# Good code - should not trigger any warnings
def good_clean_function(data):
    """Well-written function with proper practices"""
    if not data:
        return None
    
    # Proper parameterized query approach
    safe_query = "SELECT * FROM users WHERE id = ? AND status = ?"
    return {"query": safe_query, "params": [data.get("id"), "active"]}

# Mixed complexity - moderate nesting (should be OK - 5 levels <= 6)
def moderate_complexity(items):
    result = []
    for item in items:  # Level 1
        if item.get("active"):  # Level 2
            for category in item.get("categories", []):  # Level 3
                if category.get("enabled"):  # Level 4
                    for tag in category.get("tags", []):  # Level 5
                        result.append(tag)  # Still within limit
    return result