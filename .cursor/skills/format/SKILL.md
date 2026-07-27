---
name: format
description: >
  Run auto-formatters for every language touched by staged (or recently changed)
  files before committing. Detects which subsystems are affected and runs only
  the relevant formatters. Use when the user says "format", "run formatters",
  "fmt", "run format before commit", or when about to commit and formatting may
  be needed.
# Pointer to .claude/skills/format/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
---

The full skill definition is in `.claude/skills/format/SKILL.md`.
