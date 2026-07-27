---
name: test-coverage-mapper
description: Maps old driver test files and test case names (ODBC/JDBC/Python) to Universal Driver equivalents in tests/oldTestsCoverage/ YAML files, and verifies coverage via assertion-level code analysis. Use when the user says: 'map old tests', 'map old ODBC/JDBC/Python test', 'which old tests are unmapped', 'coverage gaps for ODBC/JDBC/Python', 'add test mapping', 'add a test mapping to odbc.yaml', 'reverse lookup', 'sync test mappings', 'which UD tests cover this old test', 'update the coverage yaml', 'reconcile summary counts in the yaml', 'TC_AUTH_NNN shows unmapped'. NOT for Jira bug tickets (use bug-regression-coverage for SNOW-XXXXXX). NOT for code coverage metrics such as line coverage, branch coverage, jacoco reports, coverage percentages, or coverage of newly written features.
# Pointer to .claude/skills/test-coverage-mapper/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
---

The full skill definition is in `.claude/skills/test-coverage-mapper/SKILL.md`.
Read that file for the complete workflow, output format, and quality rules.
