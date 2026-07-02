# Lifecycle README — canonical source

The lifecycle README is **not a template**. The Skills<>Evals
convention is identical across every repo that adopts it, so every
repo gets a byte-identical copy of the same file. There is exactly
one canonical source; drift is avoided by copying verbatim rather
than rendering from placeholders.

## Canonical location

`.claude/skills/README.md` in snowdev (the repo that hosts the
canonical source of this skill). Read it at runtime — do not embed
its contents in this reference file, because any drift here would
silently diverge from the real source.

## Installation

During Phase 3 (proposal) and Phase 4 (plan) of
`bootstrap-skills-evals`:

1. Read snowdev's `.claude/skills/README.md`.
2. Include its full content in the manifest as the proposed
   content for `<target-repo>/.claude/skills/README.md`.
3. The downstream agent (Phase 5 handoff via
   `sf ai agent run --deep-plan`) writes it verbatim.

No placeholder substitution. No per-repo customization. If a
target repo later wants a repo-specific addendum, that's a
post-bootstrap edit the repo owns.

## Idempotency

Same rules as every other bootstrap artifact:

- **File already exists, byte-identical to snowdev's canonical
  README** → skip.
- **File exists, diverged** → ask the user: overwrite / keep /
  merge. Never silent-overwrite.
- **File doesn't exist** → create from snowdev's canonical source.

## Why this isn't a template

The earlier design had this file host a template body with a
`ADJUST-PATH` placeholder for sf-cli install docs. That turned out
to be the only repo-variable piece, and it's better handled by the
README itself stating "sf-cli must be on PATH; see your repo's
top-level docs for install" — a generic instruction that doesn't
need per-repo rewriting.

Keeping the template in this reference file risked drift between
the reference and snowdev's actual README. The single-source-of-
truth approach (read snowdev at runtime) avoids the drift class
entirely.
