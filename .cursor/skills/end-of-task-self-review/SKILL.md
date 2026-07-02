---
name: end-of-task-self-review
description: Self-review checklist run before declaring a coding task done, opening a PR, or pushing the final commit. Catches commit-scope drift, loose test assertions, refactor-stale comments, error-string changes that need callouts, test fixtures that self-sabotage, dead-code helpers, and signature smells. Use after implementing a feature, completing a bug fix, finishing a refactor, or whenever about to write a commit message or open a PR. Distilled from review findings on the OAuth-core PR series.
# Pointer to .claude/skills/end-of-task-self-review/SKILL.md (canonical source).
# Skills fire on demand — when invoked the agent reads the canonical file fresh, so
# a pointer here is safe. This avoids duplicating ~180 lines of content.
# WHY alwaysApply rules cannot use the same approach: see .cursor/rules/*.mdc frontmatter.
---

The full skill definition is in `.claude/skills/end-of-task-self-review/SKILL.md`.
Read that file for the complete checklist and how-to instructions.
