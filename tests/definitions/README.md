# Test Definitions

This directory contains Gherkin feature files that define test scenarios for the Universal Driver across multiple languages. Tests are categorized into **E2E (end-to-end)** and **Integration** tests.

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
