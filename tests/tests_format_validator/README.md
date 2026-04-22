# Tests Format Validator

Validates that Gherkin feature files have corresponding test implementations across all supported languages.

## Feature file location

All feature files must live under `tests/definitions/shared/`. Language-specific subfolders (`core/`, `python/`, etc.) are not supported.

## Tag System

**Scenario-level tags** specify which languages/levels to test (required on each scenario):
- `@core_e2e` / `@core_int` - Rust in `sf_core/tests/e2e/` or `integration/`
- `@jdbc_e2e` / `@jdbc_int` - JDBC in `jdbc/.../e2e/` or `integration/`
- `@odbc_e2e` / `@odbc_int` - ODBC in `odbc_tests/tests/e2e/` or `integration/`
- `@python_e2e` / `@python_int` - Python in `python/tests/e2e/` or `integ/`

**Feature-level tags** (optional):
- Generic language tags: `@core`, `@jdbc`, `@odbc`, `@python` - indicate planned implementations (TODOs)
- Exclusion tags: `@core_not_needed`, `@jdbc_not_needed`, etc. - exclude languages entirely
- ⚠️ Level-specific tags (`@core_e2e`, `@core_int`) NOT allowed at feature level

**Scenario-level exclusions** (optional):
- `*_not_needed` tags can exclude specific languages per scenario

Examples:
```gherkin
@core @python
Feature: PUT/GET operations
  # Indicates Rust and Python implementations planned

  @core_e2e @python_e2e
  Scenario: Upload file
```

```gherkin
@jdbc_not_needed
Feature: Python datetime handling
  # JDBC excluded for entire feature

  @python_e2e @core_int
  Scenario: Handle timezone-aware datetime
```

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

# Run with JSON output (includes Behavior Difference data)
cargo run -- --json

# Show help
cargo run -- --help
```

## What it validates

- ✅ All feature files are in `tests/definitions/shared/`
- ✅ Each scenario has corresponding test files in required languages (from scenario tags)
- ✅ Test methods match scenario names
- ✅ All Gherkin steps are implemented as comments in test methods
- ✅ Tests are in correct directory (`_int` → integration/, `_e2e` → e2e/)
- ✅ Feature-level tags are only generic (`@core`, `@python`) or exclusions (`@*_not_needed`)
- ✅ Feature declares language but scenarios have no level tags → validation error
- ✅ Feature has `@{language}_not_needed` but scenario has `@{language}_e2e` → validation error
- ✅ Every test method in e2e and integration dirs has at least one non-empty `When` and `Then` step comment
- ⚠️ Reports orphaned test files and missing test methods

## Output

By default only failures are printed. Use `--verbose` to see all features including passing ones, and to show the list of implemented steps per validation.

- ✅ Successfully validated test implementations
- ❌ Missing implementations or validation failures
- ⚠️ Issues: validation errors (wrong directory), missing methods, missing steps
- 🔍 Orphaned tests (no Gherkin definition)
