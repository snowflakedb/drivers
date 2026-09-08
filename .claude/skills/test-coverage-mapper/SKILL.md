---
name: test-coverage-mapper
description: >
  Maps old driver test files and test case names (ODBC/JDBC/Python) to Universal Driver equivalents in tests/oldTestsCoverage/ YAML files, and verifies coverage via assertion-level code analysis. Use when the user says: 'map old tests', 'map old ODBC/JDBC/Python test', 'which old tests are unmapped', 'coverage gaps for ODBC/JDBC/Python', 'add test mapping', 'add a test mapping to odbc.yaml', 'reverse lookup', 'sync test mappings', 'which UD tests cover this old test', 'update the coverage yaml', 'reconcile summary counts in the yaml', 'TC_AUTH_NNN shows unmapped'. NOT for Jira bug tickets (use bug-regression-coverage for SNOW-XXXXXX). NOT for reviewing the current branch/diff for old-driver behavior gaps (use behavior-compatibility-review). NOT for code coverage metrics such as line coverage, branch coverage, jacoco reports, coverage percentages, or coverage of newly written features.
---

This content lives in the knowledge base. See `/sdet-non-monorepo:kb tools old-driver-coverage-mapping` —
`snowflake-eng/dev-platform-kb/.ai/commands/sdet-non-monorepo/kb/references/tools/old-driver-coverage-mapping.md` — and the concept it rests on,
`snowflake-eng/dev-platform-kb/.ai/commands/sdet-non-monorepo/kb/references/concepts/old-driver-coverage-attribution.md`.

<!-- Retire this redirect once no citation of `test-coverage-mapper` remains:
     grep -rn 'test-coverage-mapper' --include='*.md' . -->
