---
name: configure-skill-settings
description: Audits SKILL.md frontmatter across .claude/skills/ directories. Invoked when the user says "audit my skill settings", "review skill frontmatter", "configure skill settings".
# Pointer to .claude/skills/configure-skill-settings/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/configure-skill-settings/SKILL.md`.
