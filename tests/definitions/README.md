# Test Definitions

This directory contains Gherkin feature files that define test scenarios for the Universal Driver across multiple languages. Tests are categorized into **E2E (end-to-end)** and **Integration** tests.

## Directory Structure

All feature files must live under `shared/`:

- **`shared/`** - All test feature definitions
  - `authentication/`, `http/`, `put_get/`, `query/`, `session/`, `tls/`, `types/`

Language-specific directories (`core/`, `python/`, `odbc/`, `jdbc/`) are **not allowed**. Non-shared (language-specific) test Gherkin steps should be added directly in test files as comments, not in separate feature files.

## Test Types

### E2E Tests
- Tests that require connection to Snowflake deployment

### Integration Tests
- Tests that are testing multiple layers, but are not connecting to Snowflake

## Annotations

### Feature Level
- **Required**: `@{driver}` - Specifies which drivers should implement this feature
  - **Example**: `@core @python`
- **Exclusions**: `@{driver}_not_needed` - Excludes ALL scenarios in this feature for the specified driver
  - **Example**: `@core_not_needed` means no Core (Rust) tests needed for this feature

**Feature Level Behavior:**
- **If feature has NO driver annotation**: All scenarios marked as "TODO" by default
- **Feature-level exclusion**: `@{driver}_not_needed` on feature excludes ALL scenarios for that driver

### Scenario Level
- **Test Types**: `@{driver}_{test_type}` - Specifies driver and test type
  - **Test Types**: `_e2e` (end-to-end), `_int` (integration), `_unit` (unit)
  - **Examples**: `@core_e2e`, `@python_int`
- **Exclusions**: `@{driver}_not_needed` - Excludes scenario for specific driver
  - **Example**: `@python_not_needed`

**Scenario Level Behavior:**
- **If feature has driver annotation but scenario doesn't**: Scenario marked as "TODO"
- **Scenario-level exclusion**: `@{driver}_not_needed` on scenario excludes only that scenario
- HTML Report: Shows "-" when excluded, "TODO" when expected but not implemented
- Coverage calculations include TODO scenarios as expected implementations

### Language-Specific Scenarios

If a scenario is truly language-specific (e.g., Python tuple/list handling, ODBC SQLSetConnectAttr forwarding), it should **not** be in the shared feature file. Instead, move the test to a separate driver-specific test file and add Gherkin steps as comments directly in the test code. The validator will not track these.

If a scenario has a single-language tag but the behavior **could be shared** (other drivers just haven't implemented it yet), keep it in the shared feature file with the appropriate tag. Other drivers will see it as "TODO" in coverage reports.

## Validator & HTML Report Flow

1. **Validator** (`tests_format_validator/`)
   - Ensures every Gherkin scenario for which a driver-specific annotation is added has a corresponding test method implementation with correct name and comments containing Gherkin steps
   - Validates that all feature files are under `shared/`
   - Detects orphaned test files (tests with no matching feature scenario)
   - Checks that every test method has at least one `When` and `Then` step comment

2. **Coverage Report** (`tests/test_coverage_report/`)
   - Creates interactive HTML dashboards showing test coverage status and Behavior Difference annotations for easy visualization

## Adding New Tests

1. **Choose location** - All feature files go in `shared/{category}/` (e.g., `shared/authentication/`)
2. **Write the feature file** - Create a `.feature` file with Gherkin scenarios
3. **Add appropriate tags**:
   - Tag feature with `@{driver}` (e.g., `@core`, `@python`, or `@core @python`)
   - Use `@{driver}_not_needed` to exclude drivers that don't need this feature
   - Tag scenarios with `@{driver}_{test_type}` format (`_e2e`, `_int`, or `_unit`)
4. **Implement tests** - Write tests with corresponding test steps added as comments in each tagged driver's test suite:
   - **E2E tests**: use `e2e/` directories
   - **Integration tests**: use `integration/` directories
5. **Run validator** - Use the format validator to check all scenarios have matching implementations (it is added to pre-commit)

### Adding Language-Specific Tests

If a test is specific to one driver and does not belong in a shared feature:

1. Add the test directly in the driver's test directory
2. Include Gherkin step comments (`// Given`, `// When`, `// Then`) in the test method
3. No feature file is needed — the validator does not track these

## Behavior Differences (BD)

Behavior Differences document changes in driver behaviour between New and Old drivers.
Each Behaviour Difference will have separate assertions for New and Old drivers.

### BD Types

Behavior Differences are categorized into three types:

1. **Breaking Change**
2. **Bug Fix**
3. **New Feature**

### YAML Structure

Each driver has a `BehaviorDifferences.yaml` file that defines its behavior differences:

- **Root key**: `behaviour_differences`
- **Numbered entries**: Each BD is numbered sequentially (1, 2, 3, etc.)
- **Required fields**:
  - `name`:  Description of the behavior difference
- **Optional fields**:
  - `type`: One of "Breaking Change", "Bug Fix", or "New Feature"
  - `description`: Detailed explanation

### Default Behavior

When no `type` is specified in the YAML:
- The BD is displayed as **"[Behaviour Difference]"** in reports

### Test Implementation

Behavior Differences are referenced in test code using the format `BD#{number}`:

```python
# Python example
if OLD_DRIVER_ONLY("BD#1"):
    assert downloaded_content != reference_content

if NEW_DRIVER_ONLY("BD#1"):
    assert downloaded_content == reference_content
```

```cpp
// C++ example
OLD_DRIVER_ONLY("BD#1") {
    CHECK(downloaded_bytes != reference_bytes);
}

NEW_DRIVER_ONLY("BD#1") {
    CHECK(downloaded_bytes == reference_bytes);
}
```

### Coverage Report Integration

- **BD Detection**: The validator automatically detects `BD#` references in test files

### Adding New Behavior Differences

1. **Update YAML**: Add new entry to the driver's `BehaviorDifferences.yaml` file
2. **Implement Tests**: Add `BD#` references in test methods using `OLD_DRIVER_ONLY()` and `NEW_DRIVER_ONLY()` macros
3. **Run Validator**: Ensure the BD is detected and appears in coverage reports
4. **Verify Report**: Check that the BD appears correctly in the HTML coverage report

## Gherkin Best Practices

### Structure
- **Descriptive scenario names** - Use "should" statements
- **Clear Given-When-Then flow** - Setup → Action → Verification
- **Preferably one WHEN per scenario** - Each scenario should test one specific action (some exceptions for tests with long setup steps could be allowed)
- **Every test method must have at least one When and Then step** - The validator enforces this
- **No empty steps** - Every step comment must be followed by implementation code before the next step comment
