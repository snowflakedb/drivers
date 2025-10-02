#!/usr/bin/env python

import sys
import subprocess
from pathlib import Path

from config import config
from real_snowflake_compatibility_layer import install_compatibility_layer


def check_dependencies():
    print("Checking dependencies...")
    
    required_packages = ['pytest']
    missing_packages = []
    
    for package in required_packages:
        try:
            __import__(package)
            print(f"{package} is available")
        except ImportError:
            missing_packages.append(package)
            print(f"{package} is missing")
    
    if missing_packages:
        print(f"\nMissing packages: {', '.join(missing_packages)}")
        print("Please install them with: pip install pytest")
        return False
    
    return True




def test_compatibility_layer():
    print("Testing compatibility layer...")
    
    try:
        install_compatibility_layer()
        
        # Test basic imports that should work
        from snowflake.connector import connect, OperationalError, Error
        
        print("Basic imports working")
        print("Simple redirect functional")
        print("Compatibility layer test passed")
        return True
        
    except Exception as e:
        print(f"Compatibility layer test failed: {e}")
        return False


def main():
    print("SNOWFLAKE TEST COMPATIBILITY SETUP")
    print("=" * 50)
    
    print("Validating environment...")
    try:
        config.validate()
        print("Environment validation passed")
    except Exception as e:
        print(f"Environment validation failed: {e}")
        print("\nSetup tips:")
        print("- Make sure universal-driver is built: cd universal-driver && cargo build")
        print("- Ensure snowflake-connector-python is in the parent directory")
        print("- Check that all paths are accessible")
        return 1
    
    if not check_dependencies():
        return 1
    
    if not test_compatibility_layer():
        return 1
    
    print("\n" + "=" * 50)
    print("SETUP COMPLETED SUCCESSFULLY!")
    print("=" * 50)
    print("\nUsage examples:")
    print("python runner.py /path/to/snowflake-connector-python/test/unit/test_file.py::test_method -v")
    print("python runner.py /path/to/snowflake-connector-python/test/unit/test_file.py -v")
    print("python runner.py --validate-only /dev/null")
    print("\nUse 'python runner.py --help' for more options")
    print("=" * 50)
    
    return 0


if __name__ == "__main__":
    exit_code = main()
    sys.exit(exit_code)
