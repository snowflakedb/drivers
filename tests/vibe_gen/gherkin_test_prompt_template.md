# Generate Gherkin Test for ODBC Function

Generate comprehensive Gherkin test scenarios for the following ODBC function:

## Function Details

**Function Name:** {{FUNCTION_NAME}}

**Return Type:** {{RETURN_TYPE}}

**Parameters:**
{{PARAMETERS}}

## Requirements

1. Create Gherkin feature file with multiple test scenarios
2. Cover the following test cases:
   - Happy path (successful execution)
   - Invalid parameters
   - Null pointer handling
   - Boundary conditions
   - Error conditions
3. Use proper Given-When-Then structure
4. Include realistic test data
5. Consider ODBC state transitions and handle types

## Output Format

Please create a complete Gherkin feature file following this structure:

```gherkin
Feature: [Function Name] Tests
  As an ODBC application developer
  I want to test [function name]
  So that I can ensure proper database operations

  Scenario: [Scenario name]
    Given [preconditions]
    When [action]
    Then [expected result]
```

Include at least 5 different scenarios covering various use cases and edge cases.

# Guidelines
- Place the test in tests/definitions/odbc/vibe/{{FUNCTION_NAME}}
- Write similar gherkins to ones placed here: tests/definitions/core, tests/definitions/shared
- All the tests should be tagged with @odbc_vibe and feature with @odbc
- Search internet for usages of the functions to get better understanding how should tests look like
