#!/usr/bin/env python3
"""
Clean code test - should not trigger any security issues
"""

def calculate_sum(a: int, b: int) -> int:
    """Calculate the sum of two integers."""
    return a + b

def get_greeting(name: str) -> str:
    """Get a personalized greeting."""
    if not name:
        return "Hello, Guest!"
    return f"Hello, {name}!"

if __name__ == "__main__":
    result = calculate_sum(5, 3)
    greeting = get_greeting("World")
    print(f"{greeting} The sum is {result}")