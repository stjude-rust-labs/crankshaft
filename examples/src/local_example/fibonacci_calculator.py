#!/usr/bin/env python3
"""
Fibonacci calculator to find the next Fibonacci number after a given input.

This script efficiently computes Fibonacci numbers using an iterative approach
and finds the smallest Fibonacci number that is greater than the input.

Usage:
    python3 fibonacci_calculator.py <number>
    
Example:
    python3 fibonacci_calculator.py 12345
"""

import sys
import math

def find_next_fibonacci(n):
    """
    Find the next Fibonacci number after the given number n.
    
    Args:
        n (int): The number after which to find the next Fibonacci number
        
    Returns:
        int: The smallest Fibonacci number greater than n
    """
    if n < 0:
        return 0
    if n < 1:
        return 1
    
    # Start with first two Fibonacci numbers
    fib_prev, fib_curr = 0, 1
    
    # Generate Fibonacci numbers until we find one greater than n
    while fib_curr <= n:
        fib_prev, fib_curr = fib_curr, fib_prev + fib_curr
    
    return fib_curr

def is_fibonacci(n):
    """
    Check if a number is a perfect Fibonacci number.
    Uses the mathematical property that n is Fibonacci if one of
    5*n^2 + 4 or 5*n^2 - 4 is a perfect square.
    """
    def is_perfect_square(x):
        if x < 0:
            return False
        root = int(math.sqrt(x))
        return root * root == x
    
    return is_perfect_square(5 * n * n + 4) or is_perfect_square(5 * n * n - 4)

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 fibonacci_calculator.py <number>", file=sys.stderr)
        sys.exit(1)
    
    try:
        input_number = int(sys.argv[1])
    except ValueError:
        print(f"Error: '{sys.argv[1]}' is not a valid integer", file=sys.stderr)
        sys.exit(1)
    
    if input_number < 0:
        print("Error: Input must be a non-negative integer", file=sys.stderr)
        sys.exit(1)
    
    # Find the next Fibonacci number
    next_fib = find_next_fibonacci(input_number)
    
    # Check if the input itself is a Fibonacci number
    is_input_fib = is_fibonacci(input_number)
    
    # Output results
    print(f"Input number: {input_number}")
    print(f"Is Fibonacci: {is_input_fib}")
    print(f"Next Fibonacci: {next_fib}")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
