---
name: run-skill-evals
description: Runs routing-accuracy evals via sf ai skills eval. Invoked when the user says "run skill evals", "test skill routing".
# Pointer to .claude/skills/run-skill-evals/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/run-skill-evals/SKILL.md`.
