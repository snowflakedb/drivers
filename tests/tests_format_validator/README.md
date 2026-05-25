# Tests Format Validator

Validates that Gherkin feature files have corresponding test implementations across all supported languages. Runs as a pre-commit hook (`tests-format-validator`) on every commit.

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

## Alignment Targets

In addition to the tag-based validation above, the validator supports **alignment targets** — direct pairings between a features directory and a test directory that bypass the tag system. This is useful for secondary test suites (e.g., framework-specific tests) that cover the same Gherkin scenarios as the primary suite but live in a different directory.

Targets are defined in `alignment_targets.toml`:

```toml
[[target]]
name     = "pandas-types"
language = "python"
features = "tests/definitions/shared/types"
tests    = "python/tests/e2e/pandas/types"
```

Each target pairs every `.feature` file in the `features` directory with the corresponding test file in the `tests` directory (matched by stem: `boolean.feature` ↔ `test_boolean.py`). The validator then checks:
- Every scenario in the feature has a matching test method
- Every test method maps back to a scenario (no orphans)
- Step comments inside each test method match the Gherkin steps

## Writing Tests That Pass Validation

### Method naming

Test method names must match scenario names after normalization. The normalizer strips prefixes, converts to snake_case, and removes spaces, underscores, hyphens, angle brackets, and parentheses before comparing.

For **Scenario Outlines** with `<placeholder>` parameters, the angle brackets are stripped during normalization, so the placeholder name becomes part of the snake_case method name.

### Step comments

Every Gherkin step (`Given`, `When`, `Then`, `And`, `But`) must appear as a comment in the corresponding test method. 
The validator compares step text after normalizing case and stripping punctuation (`"`, `'`, `,`, `.`, `:`, `;`, `!`, `?`, `(`, `)`).

```python
def test_should_cast_boolean_values_to_appropriate_type(self, cursor):
    # Given Snowflake client is logged in      ← matches "Given Snowflake client is logged in"
    pass

    # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
    df = execute_and_fetch(cursor, "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN")

    # Then All values should be returned as appropriate type
    assert_dtypes(df, [np.bool_, np.bool_, np.bool_])

    # And Values should match [TRUE, FALSE, TRUE]
    assert get_row(df, 0) == [True, False, True]
```

**Multi-line step comments** are supported. A continuation line is any comment that immediately follows a step comment and does not start with a new Gherkin keyword:

```python
# When Query "SELECT -99999999999999999999999999999999999999::<type>,
#   99999999999999999999999999999999999999::<type>" is executed
```

### Empty steps

A step comment with no code between it and the next step (or end of method) is flagged as "empty".

### Required When/Then

Every test method must have at least one non-empty `When` and one non-empty `Then` step comment.

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
- ✅ Test methods match scenario names (normalized comparison)
- ✅ All Gherkin steps are implemented as comments in test methods
- ✅ Tests are in correct directory (`_int` → integration/, `_e2e` → e2e/)
- ✅ Feature-level tags are only generic (`@core`, `@python`) or exclusions (`@*_not_needed`)
- ✅ Feature declares language but scenarios have no level tags → validation error
- ✅ Feature has `@{language}_not_needed` but scenario has `@{language}_e2e` → validation error
- ✅ Every test method in e2e and integration dirs has at least one non-empty `When` and `Then` step comment
- ✅ Alignment targets: feature ↔ test file pairing, method matching, step matching
- ⚠️ Reports orphaned test files and missing test methods

## Output

By default only failures are printed. Use `--verbose` to see all features including passing ones, and to show the list of implemented steps per validation.

- ✅ Successfully validated test implementations
- ❌ Missing implementations or validation failures
- ⚠️ Issues: validation errors (wrong directory), missing methods, missing steps
- 🔍 Orphaned tests (no Gherkin definition)

## Common Failures and Fixes

| Failure | Cause | Fix |
|---|---|---|
| `Orphan test method (no matching scenario)` | Method name doesn't match any scenario after normalization | Rename the method to match the Gherkin scenario name exactly |
| `Missing test method for scenario` | No test method found for a feature scenario | Add a test method with the correct name |
| `Missing steps in '...'` | Step comments in the test don't match the feature steps | Add/fix `# When ...` / `# Then ...` comments to match the feature file text |
| `Empty steps in '...'` | A step comment has no code between it and the next step | Add implementation code after the step comment (even `pass` counts) |
| `Orphaned Tests Found` | Test methods exist that aren't referenced by any feature | Either add the scenario to the feature file or remove/rename the test |
