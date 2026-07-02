---
name: generate-eval-set
description: Authors/updates the routing-accuracy eval set for a skill. Invoked when the user says "generate evals for my skill", "add eval set".
# Pointer to .claude/skills/generate-eval-set/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/generate-eval-set/SKILL.md`.
