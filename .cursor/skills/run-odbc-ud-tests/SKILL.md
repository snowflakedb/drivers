---
name: run-odbc-ud-tests
description: >
  Runbook for building and running ODBC Universal Driver (UD) tests. Use
  when you need to compile the ODBC Rust driver, set up the C++ test harness,
  and execute ODBC tests via run.sh or ctest. Also use for: DRIVER_PATH
  errors, cmake/ninja/make build issues, unixodbc/iodbc setup, libsfodbc not
  found, ODBC ctest failures, or run_reference.sh comparison runs.
# Pointer to .claude/skills/run-odbc-ud-tests/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/run-odbc-ud-tests/SKILL.md`.
