---
name: behavior-compatibility-review
description: >
  Reviews driver changes against old/legacy behavior for BDs, implementation
  gaps, and test gaps. Run /behavior-compatibility-review before pushing JDBC,
  Python, ODBC, Node.js, or sf_core changes. Not suite mapping, Jira bug
  coverage, or line coverage.
argument-hint: "[optional: staged | range]"
disable-model-invocation: true
# Pointer to .claude/skills/behavior-compatibility-review/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe.
---

The full skill definition is in `.claude/skills/behavior-compatibility-review/SKILL.md`.
Read that file for the complete workflow, output format, and quality rules.
