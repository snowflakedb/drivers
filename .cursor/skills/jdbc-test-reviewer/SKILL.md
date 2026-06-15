---
name: jdbc-test-reviewer
description: Reviews JDBC/Java test code for quality, flakiness, and correctness. Invoked when the user says "review jdbc tests", "review this jdbc test", "is this jdbc test flaky", "check this java test for flakiness", "jdbc test review", or "review my jdbc test file".
# Pointer to .claude/skills/jdbc-test-reviewer/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/jdbc-test-reviewer/SKILL.md`.
