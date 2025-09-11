# Tests Format Validator

Validates that Gherkin feature files have corresponding test implementations across all supported languages.

## Usage

```bash
# Run validator from project root
./tests/tests_format_validator/run_validator.sh

# Run validator directly from this directory
cd tests/tests_format_validator
cargo run

# Run with custom paths
cargo run -- --workspace /path/to/workspace --features /path/to/features

# Run with verbose output
cargo run -- --verbose

# Run with JSON output (includes Breaking Change data)
cargo run -- --json

# Show help
cargo run -- --help
```

## What it validates

- ✅ Each `.feature` file has corresponding test files in all required languages
- ✅ Test methods match scenario names (converted to appropriate naming conventions)
- ✅ All Gherkin steps are implemented as comments in test methods
- ⚠️ Identifies orphaned test files that don't match any feature
- ⚠️ Reports missing test methods for scenarios

## Output

The validator provides colored output showing:
- ✅ Successfully validated test implementations
- ❌ Missing implementations or files
- 🔍 Orphaned Tests (No Gherkin definition)
