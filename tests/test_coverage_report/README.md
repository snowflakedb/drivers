# Test Coverage Report Generator

## Usage

```bash
# Generate HTML coverage report (default, saves to universal_driver_e2e_test_coverage.html)
python3 tests/test_coverage_report/coverage_report.py

# Generate a table format report (prints to console)
python3 tests/test_coverage_report/coverage_report.py --format table

# Generate HTML report with custom output file
python3 tests/test_coverage_report/coverage_report.py --output custom_report.html

# Specify workspace directory (if running from different location)
python3 tests/test_coverage_report/coverage_report.py --workspace /path/to/workspace
```
