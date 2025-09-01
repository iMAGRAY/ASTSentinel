#!/usr/bin/env python3
"""
Perfect clean code example with best practices
"""
import os
from typing import List, Optional


def add_numbers(a: int, b: int) -> int:
    """Add two integers safely.
    
    Args:
        a: First integer
        b: Second integer
        
    Returns:
        Sum of the two integers
    """
    return a + b


def process_items(items: List[str]) -> List[str]:
    """Process a list of items with validation.
    
    Args:
        items: List of strings to process
        
    Returns:
        Processed list of strings
    """
    if not items:
        return []
    
    processed = []
    for item in items:
        if item and item.strip():
            processed.append(item.strip().lower())
    
    return processed


def get_config_value(key: str) -> Optional[str]:
    """Safely get configuration value from environment.
    
    Args:
        key: Configuration key name
        
    Returns:
        Configuration value or None if not found
    """
    return os.environ.get(key)


if __name__ == "__main__":
    result = add_numbers(5, 3)
    items = process_items(["  Hello  ", "WORLD", "", "  "])
    print(f"Result: {result}, Items: {items}")