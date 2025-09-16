# Test Definitions

This directory contains Gherkin feature files that define test scenarios for the Universal Driver across multiple languages. Tests are categorized into **E2E (end-to-end)** and **Integration** tests.

## Test Types

### E2E Tests
- Tests that requires connection to Snowflake deployment

### Integration Tests  
- Tests that are testing multiple layers, but are not connecting to Snowflake

## Annotations

### Language and Test Type Tags
- `@{driver}_{test_type}` - Specifies which drivers and test types should implement the test
  - **Drivers**: `core`, `odbc`, `python`
  - **Test Types**: `e2e` (end-to-end), `int` (integration)
  - **Examples**: `@core_e2e`, `@odbc_int`, `@python_e2e`

### Breaking Change Detection
- Breaking Changes are automatically detected from test implementations for scenarios with regular driver tags

**Breaking Change Behavior:**
- Breaking Changes are detected by finding `NEW_DRIVER_ONLY("{Breaking Change_ID}")` and `OLD_DRIVER_ONLY("{Breaking Change_ID}")` annotations in test code
- Breaking Change descriptions are loaded from `{driver}/BreakingChanges.md` files
- HTML Report: Shows green checkmark with superscript Breaking Change numbers (e.g., `✓¹'²`) for scenarios with Breaking Changes
- Breaking Change tab shows detailed breakdown of all Breaking Changes for each driver

### Exclusion Tags
- `@{driver}_not_needed` - Explicitly excludes a scenario for a driver (e.g., `@python_not_needed`, `@odbc_not_needed`)

**Default Behavior:**
- **If feature has NO driver annotation**: All scenarios marked as "TODO" by default
- **If feature has driver annotation but scenario doesn't**: Scenario marked as "TODO"
- **Feature-level exclusion**: `@{driver}_not_needed` on feature excludes ALL scenarios for that driver
- **Scenario-level exclusion**: `@{driver}_not_needed` on scenario excludes only that scenario
- HTML Report: Shows "-" when excluded, "TODO" when expected but not implemented
- Coverage calculations include TODO scenarios as expected implementations

## Validator & HTML Report Flow

1. **Validator** (`tests_format_validator/`)
   - Ensures every Gherkin scenario for which driver specific annotation is added, has a corresponding test method implementation with correct name and comments containing Gherkin steps
   
2. **Coverage Report** (`tests/test_coverage_report/`)
   - Creates interactive HTML dashboards showing test coverage status and Breaking Change annotations for easy visualization

## Adding New Tests

1. **Write the feature file** - Create a `.feature` file in the appropriate category folder with Gherkin scenarios
2. **Add appropriate tags** - Tag scenarios with `@{driver}_{test_type}` format:
   - **E2E tests**: Use `_e2e` suffix
   - **Integration tests**: Use `_int` suffix
3. **Implement tests** - Write tests with corresponding test steps added as comments in each tagged driver's test suite:
   - **E2E tests**: use `e2e/` directories
   - **Integration tests**: use `integration/` directories
4. **Run validator** - Use the format validator to check all scenarios have matching implementations (it is added to pre-commit)

## Gherkin Best Practices

### Structure
- **Descriptive scenario names** - Use "should" statements
- **Clear Given-When-Then flow** - Setup → Action → Verification
- **Preferably one WHEN per scenario** - Each scenario should test one specific action (some exceptions for tests with long setup steps could be allowed)
