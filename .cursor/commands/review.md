---
description: Review changes in the PR
---

# Review 

## Run Arctic owl review

```
sf ai review run
```

## As @odbc-test-reviewer review odbc-tests

For each file, systematically check all categories (RAII, test structure, ODBC call validation, assertions, data retrieval, behavior differences, code style, abstraction levels, ODBC spec compliance, and coverage gaps).

Fetch the relevant ODBC function spec from Microsoft docs before checking spec compliance.

Output findings grouped by severity (High / Medium / Low) using the review output format defined in the rule.

## Combine review reports and present to the user

- Merge findings from the Arctic Owl review and the ODBC test review into a single report.
- Deduplicate overlapping issues, keeping the more detailed description.
- Group all findings by severity (High / Medium / Low), then by file.
- Append the ODBC Spec Compliance and Missing Test Coverage sections from the test review.
- End with the consolidated checklist below.
