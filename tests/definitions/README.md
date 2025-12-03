# Test Definitions

This directory contains Gherkin feature files that define test scenarios for the Universal Driver across multiple languages. Tests are categorized into **E2E (end-to-end)** and **Integration** tests.

## Directory Structure

Features are organized into two categories:

- **`shared/`** - Multi-language test features (implemented across multiple drivers)
  - `authentication/`, `http/`, `put_get/`, `query/`, `tls/`
- **`core/`** - Core (Rust) driver-only features (marked with `@core_only`)
- **`python/`** - Python driver-only features (marked with `@python_only`)
- **`odbc/`** - ODBC driver-only features (marked with `@odbc_only`)
- **`jdbc/`** - JDBC driver-only features (marked with `@jdbc_only`)

## Test Types

### E2E Tests
- Tests that requires connection to Snowflake deployment

### Integration Tests  
- Tests that are testing multiple layers, but are not connecting to Snowflake

## Annotations

### Feature Level
- **Required**: `@{driver}` - Specifies which drivers should implement this feature
  - **Example**: `@core @python`
- **Exclusions**: `@{driver}_not_needed` - Excludes ALL scenarios in this feature for the specified driver
  - **Example**: `@python_not_needed` means no Python tests needed for this feature

**Feature Level Behavior:**
- **If feature has NO driver annotation**: All scenarios marked as "TODO" by default
- **Feature-level exclusion**: `@{driver}_not_needed` on feature excludes ALL scenarios for that driver
- **Language-specific features**: Detected by folder location (e.g., `core/`, `python/`)
  - Features in language-specific folders can ONLY have tags for that driver
  - Example: Features in `core/` can only use `@core`, `@core_e2e`, `@core_int`
  - Excluded from cross-language coverage calculations

### Scenario Level  
- **Test Types**: `@{driver}_{test_type}` - Specifies driver and test type
  - **Test Types**: `_e2e` (end-to-end), `_int` (integration)
  - **Examples**: `@core_e2e`, `@python_int`
- **Exclusions**: `@{driver}_not_needed` - Excludes scenario for specific driver
  - **Example**: `@python_not_needed`

**Scenario Level Behavior:**
- **If feature has driver annotation but scenario doesn't**: Scenario marked as "TODO"
- **Scenario-level exclusion**: `@{driver}_not_needed` on scenario excludes only that scenario
- HTML Report: Shows "-" when excluded, "TODO" when expected but not implemented
- Coverage calculations include TODO scenarios as expected implementations

## Validator & HTML Report Flow

1. **Validator** (`tests_format_validator/`)
   - Ensures every Gherkin scenario for which driver specific annotation is added, has a corresponding test method implementation with correct name and comments containing Gherkin steps
   
2. **Coverage Report** (`tests/test_coverage_report/`)
   - Creates interactive HTML dashboards showing test coverage status and Behavior Difference annotations for easy visualization

## Adding New Tests

1. **Choose location** - Determine if the feature is shared or language-specific:
   - **Shared features**: Place in `shared/{category}/` (e.g., `shared/authentication/`)
   - **Language-specific features**: Place in `{driver}/{category}/` (e.g., `core/tls/`, `python/http/`)
2. **Write the feature file** - Create a `.feature` file with Gherkin scenarios
3. **Add appropriate tags**:
   - Tag feature with `@{driver}` (e.g., `@core`, `@python`, or `@core @python`)
   - Tag scenarios with `@{driver}_{test_type}` format (`_e2e` or `_int`)
   - **Important**: Features in language-specific folders must only use tags for that driver
     - Example: `core/` features can only have `@core` tags, not `@python`
4. **Implement tests** - Write tests with corresponding test steps added as comments in each tagged driver's test suite:
   - **E2E tests**: use `e2e/` directories
   - **Integration tests**: use `integration/` directories
5. **Run validator** - Use the format validator to check all scenarios have matching implementations (it is added to pre-commit)

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
