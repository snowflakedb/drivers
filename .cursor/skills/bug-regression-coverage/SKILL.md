---
name: bug-regression-coverage
description: Checks UD regression coverage for a specific Jira bug ticket identified by a SNOW-XXXXXX ticket number. Use when the user says: 'check bug regression', 'regression coverage for SNOW-', 'bug coverage check', 'does UD cover this bug', 'verify bug in UD', '/bug-regression-coverage'. Always requires a SNOW-XXXXXX ticket key — does NOT handle test file mapping, old test names, YAML coverage files, or coverage gap reports across driver test suites (use test-coverage-mapper for those).
# Pointer to .claude/skills/bug-regression-coverage/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/bug-regression-coverage/SKILL.md`.
