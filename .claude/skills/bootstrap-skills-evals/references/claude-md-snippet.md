# CLAUDE.md snippet — root-level pointer

The bootstrap adds a single short section to the target repo's root
`CLAUDE.md` so that agents entering the repo discover the skills
lifecycle.

## Snippet (copy verbatim)

```markdown
## Skills & Evals

This repo uses the `sf ai skills` convention. See
[.claude/skills/README.md](.claude/skills/README.md) for the full
lifecycle (author → generate-eval → check → eval → iterate).
```

## Placement

- **Preferred**: after the intro / top-level scope sections, before
  the first domain-specific section. Similar to where snowdev
  places its `## Skill Scope Rules`.
- **Alternate**: append at the end, under a conventional catch-all
  section. Acceptable but less discoverable.
- **Ask the user** during Phase 2 if neither spot is obviously
  right (e.g., the CLAUDE.md is short and heterogeneous, or the
  user has a strong opinion on structure).

The skill must show the user the first ~30 lines of their
CLAUDE.md and propose a concrete insertion point (line number),
not just "somewhere sensible."

## Idempotency

Detection is trigger-phrase-based:

- If the root CLAUDE.md already contains the phrase
  `sf ai skills` or a link to `.claude/skills/README.md`, skip.
- If it contains neither, add the snippet.
- Don't pattern-match the exact snippet text — the user may have
  adjusted wording. `sf ai skills` mention = "installed".

## Integration notes

- If the target repo's root `CLAUDE.md` doesn't exist, the skill
  should ask the user: create one with just this section, or skip?
  (Creating a stub CLAUDE.md in a repo that doesn't have one is a
  bigger decision than the user pressed "bootstrap skills" for.)
- If the repo has a different naming convention for AI-instruction
  files (e.g., `AGENTS.md`, `INSTRUCTIONS.md`), ask the user which
  file to update.
