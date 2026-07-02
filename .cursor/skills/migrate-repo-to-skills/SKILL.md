---
name: migrate-repo-to-skills
description: Plans the migration from .ai/commands/ to .claude/skills/. Invoked when the user says "migrate .ai to skills".
# Pointer to .claude/skills/migrate-repo-to-skills/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/migrate-repo-to-skills/SKILL.md`.
