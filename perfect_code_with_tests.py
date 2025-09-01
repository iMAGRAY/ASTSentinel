#!/usr/bin/env python3
"""
Perfect code with comprehensive tests and documentation
"""
import unittest
from typing import List, Optional


def add_numbers(a: int, b: int) -> int:
    """Add two integers safely.
    
    Args:
        a: First integer
        b: Second integer
        
    Returns:
        Sum of the two integers
        
    Raises:
        TypeError: If inputs are not integers
    """
    if not isinstance(a, int) or not isinstance(b, int):
        raise TypeError("Both arguments must be integers")
    return a + b


def validate_email(email: str) -> bool:
    """Validate email format using proper regex.
    
    Args:
        email: Email address to validate
        
    Returns:
        True if email format is valid, False otherwise
    """
    import re
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))


class TestMathFunctions(unittest.TestCase):
    """Test cases for mathematical functions."""
    
    def test_add_numbers_positive(self):
        """Test addition of positive numbers."""
        self.assertEqual(add_numbers(2, 3), 5)
        
    def test_add_numbers_negative(self):
        """Test addition with negative numbers.""" 
        self.assertEqual(add_numbers(-1, 1), 0)
        
    def test_add_numbers_type_error(self):
        """Test type validation."""
        with self.assertRaises(TypeError):
            add_numbers("2", 3)


class TestEmailValidation(unittest.TestCase):
    """Test cases for email validation."""
    
    def test_valid_email(self):
        """Test valid email formats."""
        self.assertTrue(validate_email("test@example.com"))
        
    def test_invalid_email(self):
        """Test invalid email formats."""
        self.assertFalse(validate_email("invalid.email"))


if __name__ == "__main__":
    unittest.main()