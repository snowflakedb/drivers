---
name: author-skill
description: Creates a new skill or modifies an existing one via LLM. Invoked when the user says "create a skill", "new skill", "fix this skill".
# Pointer to .claude/skills/author-skill/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/author-skill/SKILL.md`.
