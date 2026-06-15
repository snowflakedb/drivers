---
description: Review changes in the PR
---

# Review 

## Run Arctic owl review

```
sf ai review run
```

(Arctic Owl, the general reviewer, loads rules from `.ai/review/`)

## As @odbc-test-reviewer review odbc-tests

For each file, systematically check all categories (RAII, test structure, ODBC call validation, assertions, data retrieval, behavior differences, code style, abstraction levels, ODBC spec compliance, and coverage gaps).

Fetch the relevant ODBC function spec from Microsoft docs before checking spec compliance.

Output findings grouped by severity (High / Medium / Low) using the review output format defined in the rule.

## As @jdbc-test-reviewer review jdbc/src/test/java

When JDBC test files are in the diff, review each changed file under `jdbc/src/test/java/` for test quality, correctness, and flakiness.

Systematically check all categories in `.cursor/rules/jdbc-test-reviewer.mdc` (resource management, test structure, JDBC call validation, assertions, data retrieval, behavior differences, code style/DRY, isolation/flakiness, WireMock, and coverage gaps). Flakiness checks map to `.ai/review/universal-driver-flaky-tests.yaml` (`jdbc-*` rules and universal `ud-*` patterns).

Prioritize known hotspots: resource cleanup (try-with-resources), multistatement SQL object names, shared `getDefaultConnection()` session mutations, error-path assertions (SQLException + state/code), bare `Thread.sleep`, and WireMock port/reset setup.

Output findings grouped by severity (High / Medium / Low) using the review output format defined in the rule.

## Combine review reports and present to the user

- Merge findings from the Arctic Owl review, the ODBC test review, and the JDBC test review into a single report.
- Deduplicate overlapping issues, keeping the more detailed description.
- Group all findings by severity (High / Medium / Low), then by file.
- Append the ODBC Spec Compliance and Missing Test Coverage sections from the ODBC test review.
- Append the JDBC Flaky Test Checklist from the JDBC test review when JDBC test files were reviewed.
- End with the consolidated checklist below.
