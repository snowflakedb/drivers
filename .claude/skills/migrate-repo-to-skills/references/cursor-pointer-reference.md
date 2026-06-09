# Cross-platform bridge reference (Cursor + Codex CLI)

Claude Code walks `.claude/skills/` recursively. Cursor and Codex
CLI both walk `.agents/skills/` recursively and both explicitly
support symlinked skill folders (per their official docs). That's
the bridge we use.

For every `<module>/.claude/skills/<skill>/SKILL.md` discovered in a
repo, we create a git-tracked symlink at
`<module>/.agents/skills/<skill>` pointing to the sibling
`.claude/skills/<skill>` directory (i.e., `../../.claude/skills/<skill>`
relative to the symlink's own location). For root-level skills, the
same construction applies — the symlink lives at `.agents/skills/<skill>`
and still targets `../../.claude/skills/<skill>`, because the
symlink's parent is always `<ModuleRoot>/.agents/skills/`, two levels
above `<ModuleRoot>`.

**Use `sf ai skills bridge`** — it walks the repo, creates missing
symlinks, retargets stale ones, and sweeps orphan symlinks whose
source is gone. Do not hand-write entries under `.agents/skills/`.

## Why symlinks

The git object graph (one 120000-mode blob per symlink, ~40 bytes)
does the deduplication. No generator, no drift, no regeneration
step, no wire-shape change. Endorsed by Claude's own docs for
`.claude/rules/`; documented as first-class in Codex's skills
docs; confirmed empirically for Cursor.

## `.claude/rules/` is NOT bridged

Rules (`.claude/rules/*.md`) have no Cursor or Codex equivalent at
`.agents/rules/`. They remain Claude-only.

## Repos with existing `.agents/skills/` full-copy content

Some repos (notably snowdev's own `.agents/skills/snowci/**`) keep
full-copy directories at `.agents/skills/` as deliberate generated
output. `sf ai skills bridge` refuses to run against those by
default — the rejection is named on the offending path, and the
error points to the `--lenient` escape hatch.

Run `sf ai skills bridge --lenient` to tolerate those entries:
the tool still creates missing symlinks for sources that have no
`.agents/skills/` entry, and still sweeps orphan *symlinks*, but
leaves real files and directories alone.

Precommit hooks can opt into lenient mode by setting
`SF_AI_SKILLS_BRIDGE_LENIENT=1` in the environment — the generated
wrapper script picks it up and passes `--lenient` through.

## What NOT to do

- **Don't hand-write files or directories under `.agents/skills/`.**
  The bridge command owns this surface. Real content collides with
  the symlink (strict error, lenient skip).
- **Don't commit `.agents/skills/` bridges without running `sf ai
  skills bridge`.** The precommit hook enforces this, but direct
  git operations (`git add -A` of a partially-broken tree) can slip
  through.
- **Don't reuse the old `.cursor/rules/*.mdc` pointer convention**
  — Cursor itself is deprecating the rules surface (the Cursor
  team's `/migrate-to-skills` slash command converts dynamic rules
  to skills). Any remaining `.cursor/rules/*.mdc` in a repo should
  be removed alongside bridge adoption.

## Verification

After migration, verify bridges are complete:

```bash
sf ai skills bridge          # creates missing; exits 0 if in sync
sf ai skills bridge          # second run: no-op, confirms
sf ai skills check           # catches any bridge.* findings:
                             #   bridge.missing / bridge.not_symlink /
                             #   bridge.wrong_target / bridge.orphan
```

On a well-migrated repo, `sf ai skills bridge` at any time is a
no-op. `sf ai skills check` raises no `bridge.*` findings. Any
diff means drift to investigate.
