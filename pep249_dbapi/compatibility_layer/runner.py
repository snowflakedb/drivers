#!/usr/bin/env python3
"""
Universal test runner for running old snowflake-connector-python tests with new universal driver.

This runner installs a compatibility layer that redirects snowflake.connector imports 
to pep249_dbapi, then runs the tests in their original location.
"""

import argparse
import sys
from pathlib import Path

from config import config
from real_snowflake_compatibility_layer import install_compatibility_layer


def main():
    parser = argparse.ArgumentParser(
        description="Run old snowflake-connector-python tests with new universal driver"
    )
    
    parser.add_argument("test_path")
    
    parser.add_argument("--validate-only", action="store_true")
    
    args, pytest_args = parser.parse_known_args()
    
    
    try:
        config.validate()
    except Exception:
        return 1
    
    if args.validate_only:
        return 0
    
    test_input = args.test_path
    test_method = None
    
    if "::" in test_input:
        file_part, method_part = test_input.split("::", 1)
        test_path = Path(file_part).resolve()
        test_method = method_part
    else:
        test_path = Path(test_input).resolve()
    
    if not test_path.exists():
        return 1
    
    
    install_compatibility_layer()
    import pytest
    
    if test_method:
        final_test_spec = f"{test_path}::{test_method}"
        final_args = [final_test_spec] + pytest_args
    else:
        final_args = [str(test_path)] + pytest_args
    
    sys.exit(pytest.main(final_args))


if __name__ == "__main__":
    main()