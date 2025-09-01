#!/usr/bin/env python3
"""
Final test for clean code - should show detailed positive feedback
"""

def calculate_average(numbers):
    """Calculate average of a list of numbers.
    
    Args:
        numbers: List of numeric values
        
    Returns:
        Average value as float
    """
    if not numbers:
        return 0.0
    
    return sum(numbers) / len(numbers)


def validate_input(value):
    """Validate user input safely.
    
    Args:
        value: Input value to validate
        
    Returns:
        True if valid, False otherwise
    """
    if not isinstance(value, (int, float)):
        return False
    
    return 0 <= value <= 100


if __name__ == "__main__":
    data = [1, 2, 3, 4, 5]
    avg = calculate_average(data)
    print(f"Average: {avg}")