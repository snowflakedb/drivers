# Batch strategy — PR-stack shape and grouping rules

The migrate skill produces a PR-stack plan in Phase 4. This reference
defines the stack shape (mandatory) and the batching rules for the
N batch PRs in the middle.

## Stack shape (mandatory ordering)

The stack has exactly three kinds of PR, in this order:

### 1. First PR — deprecated-tooling sweep

Rewrites the precommit hook and removes `sf ai rules build`. Details
in `deprecated-sweep.md`. This PR MUST land before any batch PR;
otherwise every batch will fight the old hook on every commit and
spew regenerated pointer files into the diff.

Contents:
- `.pre-commit-config.yaml` rewrite (rescope `sf ai rules lint`, add
  `sf ai skills bridge` hook).
- `.pre-commit-scripts/*.sh` updates.
- Any CI/cron script updates (grep targets in `deprecated-sweep.md`).
- Doc updates that reference the old commands.

No skill migration yet. Single reviewable PR.

### 2. N batch PRs

Each ships a logical group of 15–20 skills end-to-end.

### 3. Final cleanup PR

Items in `cleanup-checklist.md`. No new skills; only leftover deletion,
orphan reference fixes, `.agents/skills/` bridge creation via
`sf ai skills bridge`, and the final `sf ai skills check`.

## Batching rules

Apply these rules to the source artifacts from Phase 1's audit:

### Primary grouping axis

Top-level subdirectory of `.ai/commands/`. This is the most natural
seam because each subdir tends to reflect a team or area of concern,
so reviewers for that subdir's batch can ack together.

### Batch size: 15–20 skills (typical)

This is a heuristic for reviewer cognitive load, not a contract.
Match the target repo's existing PR-size culture — some teams
already review 50-file PRs routinely; others keep PRs under 10
files. Audit the repo's recent PR history (or ask the user) to
calibrate before proposing the split.

Starting heuristics assuming typical 15–20-skill batches:

- **Subdirs with >20 skills**: split by second-level subdirectory,
  or by logical theme if no clean split exists.
- **Subdirs with <15 skills**: either ship alone (acceptable — a
  small batch is fine if the subdir is distinct) or pool with
  sibling subdirs of similar theme.
- **Loose root-level files**: batch together as one "miscellaneous"
  group.

If the repo's review culture points elsewhere — smaller batches
for stricter review, larger batches for a monorepo with practiced
reviewers — adjust. Just keep the other rules (stack shape, self-
consistency, topological ordering, frontmatter discipline) intact
regardless of batch size.

### Batch contents (mandatory)

Every batch PR must contain, for the skills in that batch:

1. **Moved skill dirs** — the `.claude/skills/<name>/` tree, either
   mirrored or flattened per the layout chosen in Phase 2. For each
   moved `SKILL.md`, verify the frontmatter has a top-level
   `name: <leaf-dir>` field; legacy `.ai/commands/*.md` files often
   omitted `name`, and the contract's `frontmatter.name.missing` rule
   will block `sf ai skills check` without it. Add/fix during the
   move, not after — makes the per-batch `check` diff clean.
2. **References** — `.ai/context/*.md` files consumed by these skills,
   moved to `.claude/skills/<owning-skill>/references/`. If a context
   file is shared across batches, it goes into the batch that owns the
   primary consumer; other batches point to that canonical location.
3. **Eval sets** — generate `eval_sets/routing-accuracy.yaml` for
   every non-exempt skill in the batch via `<sf> ai skills
   generate-eval`. Skills with `disable-model-invocation: true` skip
   this.
4. **Reference updates** — every file that previously pointed into
   `.ai/commands/` or `.ai/context/` for a skill in this batch must
   be updated to the new path. Targets: ancestor `CLAUDE.md`,
   `.claude/agents/**`, `.claude/commands/**`, and other markdown.
5. **Source deletion** — delete the migrated `.ai/commands/<...>.md`
   or `.ai/commands/<...>/` entries. The cleanup PR at the stack tip
   removes any stragglers, but per-batch deletion keeps each PR
   self-consistent.

**No reference-only PRs.** Every PR in the stack must either ship new
skills or do the final cleanup. A PR that only updates references to
not-yet-migrated skills is a dangling half-migration.

### Topological ordering

If skill A's SKILL.md or references/ point to skill B, B's batch must
merge before A's. Build a dependency graph from Phase 1's reference
grep and order batches accordingly.

If circular references exist between batches, break the cycle in the
smaller batch first, with a temporary placeholder pointer that the
cleanup PR removes.

### Naming convention for batch branches

`mpatankar-migrate-<topic>-<N>-of-<total>` or equivalent. The agent
that ships the stack (Phase 5 handoff) may pick its own scheme.

## Example — a medium repo with 50 skills

Audit finds:
- `.ai/commands/dev-env/` — 18 skills
- `.ai/commands/snowci/` — 22 skills
- `.ai/commands/perf/` — 4 skills
- `.ai/commands/cost-efficiency/` — 3 skills
- Root-level — 3 skills

Produced stack (6 PRs total):

1. **PR 1** — deprecated-sweep.
2. **PR 2** — dev-env/ batch (18 skills + references + eval sets).
3. **PR 3** — snowci/ first half (11 skills, picked by second-level
   dir for review locality).
4. **PR 4** — snowci/ second half (11 skills).
5. **PR 5** — perf/ + cost-efficiency/ + root (10 skills, pooled
   since each is small).
6. **PR 6** — final cleanup.

## Example — a small repo with 12 skills

- `.ai/commands/foo/` — 8 skills
- `.ai/commands/bar/` — 4 skills

Produced stack (3 PRs total):

1. **PR 1** — deprecated-sweep.
2. **PR 2** — foo/ + bar/ pooled (12 skills total, under the 20 cap).
3. **PR 3** — final cleanup.
