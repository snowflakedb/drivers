---
name: migrate-repo-to-skills
description: Migrates a repo's legacy `.ai/commands/` and `.ai/context/` directories to `.claude/skills/` + `.claude/rules/`. Use when the user says "migrate .ai to skills", "run sf ai skills migrate", "convert .ai/commands", or "port legacy commands".
argument-hint: "[repo-root]"
allowed-tools: [Bash, Read, Glob, Grep]
---

# migrate-repo-to-skills

Plan a repo's migration from the legacy `.ai/commands/` + `.ai/context/` layout to the `.claude/skills/` + `.claude/rules/` convention enforced by `sf ai skills check`. **This skill does not write files.** It audits, asks for signoff, produces a PR-stack plan, and then hands the plan to `sf ai agent run` which produces the actual PRs.

**Scope:** only `.ai/commands/` and `.ai/context/`. Never touch `.ai/review/`, `.ai/casper-tasks/`, `.ai/mcp/`, `.ai/plans/`, `.ai/README.md`, or `.ai/OWNERS.yml`.

**How this skill gets invoked:** three paths reach the same Phase 0:

1. `sf ai skills migrate` from a terminal — the CLI fetches this skill from snowdev main and runs it interactively.
2. Auto-load by `bootstrap-skills-evals` after a successful bootstrap that detects un-migrated `.ai/` content — bootstrap's "What's next" asks the user yes/no, and on `yes` loads this skill in the same session. (Recommended path for repos that have legacy `.ai/` content during onboarding — single Claude session, no terminal hop.)
3. Path B — typing "migrate .ai to skills" (or another trigger phrase from the description) inside any Claude Code session in a repo where this skill is already installed under `.claude/skills/migrate-repo-to-skills/`. Useful for re-entering the migration in a later session.

## Adaptation principle — read this before Phase 1

This skill's references (`batch-strategy.md`, `deprecated-sweep.md`, `cleanup-checklist.md`) codify what snowdev learned about safely moving large sets of rules into the skill layout. They describe **requirements** (stack shape, batch hygiene, sweep-first ordering) and **patterns**, not **exact commands to paste**.

**Before proposing a PR-stack plan, audit the repo:**

- What does the repo's review culture look like? The `batch-strategy.md` target of 15–20 skills per batch reflects typical reviewer cognitive load; teams with stricter review conventions may want smaller batches, monorepo teams may tolerate larger ones. Honor the team's existing PR-size norms.
- What deprecated tooling actually exists here? Per `deprecated-sweep.md`: `sf ai rules build` is **removed** (generates pointers into the old `.claude/` / `.cursor/` layout); `sf ai rules lint` is **rescoped, not deleted** — it still validates `.ai/review/**` Arctic Owl reviewer configs, and silently dropping that validation would let broken configs reach production. Grep first; don't assume the standard wiring is in place — custom wrapper scripts and hybrid setups happen.
- How does the repo's precommit + CI pipeline invoke rules tooling today? The first PR (deprecated-sweep) must rewrite invocations using the repo's conventions, not replace them with snowdev's wrapper script verbatim.

**Do not force snowdev's shape onto a repo that has different ownership or review norms.** A 4-PR stack that looks clean on snowdev may produce batches too large for a team with stricter review conventions, or split work a domain team would rather see together. The *stack shape* (sweep → batches → cleanup) is required — the *exact batch sizing and grouping* should fit the repo.

Execute the following steps in order.

## Workflow

### Phase 0 — Resolve `sf` binary

Load `metadata/parameters.md` and follow its instructions. Use the resulting `<sf>` path for every subsequent invocation.

### Phase 1 — Audit

Inventory the repo. Produce counts and file lists that will feed the proposal and plan phases.

1. **Source inventory.** Collect:
   - Every file under `<repo-root>/.ai/commands/` (recursive, `.md` only).
   - Every file under `<repo-root>/.ai/context/` (recursive, `.md` only).
   - Per top-level subdirectory of `.ai/commands/`: count of leaf `.md` files.
   - Per-file: is it a single `.md` or a dir containing `SKILL.md` + supporting files?

2. **Reference grep.** Find every repo file that mentions `.ai/commands/` or `.ai/context/` as a path reference. Targets:
   - Every `CLAUDE.md` in the repo (ancestor + descendant).
   - `.claude/agents/**/*.md`, `.claude/commands/**/*.md`.
   - `.claude/settings*.json`, `.claude/rules/**` if present.
   - Any other `*.md` in the repo.

   Record these — the cleanup PR will rewrite every reference.

3. **Existing skills collision check.** List every `.claude/skills/**/SKILL.md` currently in the repo. For each `.ai/commands/<x>.md` or `.ai/commands/<x>/SKILL.md`, check for a matching `.claude/skills/<x>/`:
   - **Identical** → propose deleting the `.ai/` copy in the cleanup PR.
   - **Diverged** → flag for the user. Never silent-delete. Ask which version to keep or how to merge.

4. **Deprecated-tooling sweep.** Load `references/deprecated-sweep.md` and grep the repo for `sf ai rules build` and `sf ai rules lint`. Record every hit — the first PR in the stack will rewrite them. Grep targets: `.pre-commit-config.yaml`, `.pre-commit-scripts/**`, `.github/workflows/**`, `.buildkite/**`, `**/*.py` (CI/cron), `**/docs/**/*.md`.

5. **Classification.** Load `references/decision-matrix.md`. Apply it to every source artifact from step 1 to produce a proposed target (skill / rule / CLAUDE.md entry / delete). For anything slated to become a `.claude/rules/*.md`, flag it — the user must decide per artifact in Phase 3 since rules are repo-specific policy.

### Phase 2 — Layout choice (blocking)

Before showing the full mapping table, ask the user to pick a target layout. Load `references/layout-options.md` for the exact presentation script. Three options:

- **(a) Mirror source structure** — `.ai/commands/dev-env/foo.md` → `.claude/skills/dev-env/foo/SKILL.md`. Root-level files stay at `.claude/skills/<name>/`.
- **(b) Flatten to root** — everything at `.claude/skills/<name>/`. Collisions between two source subdirs get hybrid-nested: only the colliding basenames go into subdirs, everything else stays flat.
- **(c) Custom** — the proposal table lets the user edit per row.

**No default.** Always ask. The user must pick explicitly. Show the skill count and per-source-subdir breakdown to inform the choice, but don't recommend — the right answer depends on the team's ownership model and repo size.

### Phase 3 — Propose (blocking)

Render the full mapping table using the layout chosen in Phase 2. Columns: `Source → Target → Action → Rationale`.

For any row targeting `.claude/rules/*.md`, ask the user which rule file the content should go into (existing or new). Do not guess.

For any row flagged as diverging from an existing `.claude/skills/` entry, ask the user which version to keep or how to merge.

Block on explicit signoff: "Approve proposal? y/n/edit". If "edit", iterate — re-present the table and re-ask. Never proceed without explicit approval.

### Phase 4 — Plan

Produce a PR-stack plan as a single markdown document. Stack shape (mandatory):

1. **First PR — deprecated-tooling sweep.** Rewrites the precommit hook and removes `sf ai rules build`. Details in `references/deprecated-sweep.md`. This PR MUST land before any batch PR so downstream PRs don't fight the deprecated hook on every commit.

2. **N batch PRs** — 15–20 skills per batch, logically grouped by source top-level dir per `references/batch-strategy.md`. Each batch PR ships:
   - Moved skill dirs.
   - Their `references/` (pulled from `.ai/context/` where applicable).
   - Eval sets via `<sf> ai skills generate-eval` for non-exempt skills.
   - All reference updates (CLAUDE.md, `.claude/agents/**`, `.claude/commands/**`, `.cursor/skills/`) for skills in this batch.
   - No reference-only PRs — every PR must be self-consistent.

3. **Final cleanup PR** — items in `references/cleanup-checklist.md`: `.agents/skills/` bridge creation (via `<sf> ai skills bridge`), orphan reference fixes, `.ai/commands/` + `.ai/context/` deletion, final `sf ai skills check` must pass.

Write the full plan to stdout so the user sees it before handoff.

### Phase 5 — Handoff

If the user invoked with `--dry-run` in the prompt text, stop here and report the plan path. Do NOT proceed to the agent handoff.

Otherwise, pipe the plan to `sf ai agent run`:

```
echo "<plan>" | <sf> ai agent run --deep-plan
```

Capture and report the agent's output (PR URLs / branch names). `sf ai agent run` may background — don't block waiting for completion.

### Phase 6 — Verify (post-agent)

When the agent completes, re-run `<sf> ai skills check <repo-root>` and diff against the pre-migration baseline captured in Phase 1. Report:

- Regressed findings (if any).
- Confirmation that `.ai/commands/` and `.ai/context/` are empty or deleted.
- Bridge audit: every `.claude/skills/<module>/<skill>/SKILL.md` has a matching symlink at `<module>/.agents/skills/<skill>` pointing to `../../.claude/skills/<skill>`. Run `<sf> ai skills bridge --help` if the user hasn't wired it into precommit yet.
- **Eval-set gaps** surfaced by `<sf> ai skills status`. Post-migration, most newly-migrated skills won't have eval sets yet — that's expected. Surface the list of skills missing eval sets so the user knows the next phase of work: running `<sf> ai skills generate-eval <skill-dir>` for each one. Honor the adopter repo's PR conventions when planning how to batch those eval-set commits — ask the user whether to bundle by area, by owner, or one-per-skill.
- **Frontmatter hygiene**: point the user at `configure-skill-settings` (the audit meta-skill installed by `repo-setup`). Migration moves content into place but doesn't normalize frontmatter — `configure-skill-settings` flags `disable-model-invocation` mistakes, paths vs globs, description length violations, and name/directory mismatches. Worth a run after the migration stack lands.

## Output format

Always end with:

1. **Phase reached** — 1/2/3/4/5/6.
2. **Proposal summary** — total skills, target layout chosen, any diverging-skill flags.
3. **Plan path** (if Phase 4 reached) — where the PR-stack plan markdown lives.
4. **PR URLs** (if Phase 5 completed) — output from `sf ai agent run`.
5. **Final check delta** (if Phase 6 completed) — diff against pre-migration baseline.
6. **What's next** — the migration produces a PR stack that lands over multiple sessions; name the user's immediate next action:
   - *Each PR in the stack must merge in topological order (sweep → batch 1 → batch 2 → … → cleanup). Wait for each to merge before the next opens. Come back for the next batch when ready.*
   - *After the full stack merges, re-run `sf ai skills check` and `sf ai skills status` to see post-migration state, then run `sf ai skills generate-eval <skill-dir>` for each skill `status` flags as missing an eval set.*
   - *PR shape for the eval-set gap-fill phase depends on the repo's review culture — ask the user whether to batch by area, by owner, or one per skill. Don't assume.*

## Quality rules

- **Never create `.claude/rules/*.md` without explicit user choice.** Rules are repo-specific policy; guessing produces wrong answers that are hard to unwind.
- **Never touch `.ai/review/`, `.ai/casper-tasks/`, `.ai/mcp/`, `.ai/plans/`, `.ai/README.md`, `.ai/OWNERS.yml`.** Scope is strictly `commands/` + `context/`.
- **Never silent-delete.** If an `.ai/commands/<x>.md` looks like a duplicate of an existing `.claude/skills/<x>/SKILL.md`, diff them. If identical, propose the delete. If diverged, ask.
- **Never skip the layout question** (Phase 2). No sensible default; always ask.
- **Never exceed the batch size cap** (15–20 skills per batch). A single 200-skill PR is unreviewable and forbidden.
- **Never let `sf ai rules build` survive the migration.** If the deprecated-sweep PR doesn't land first, batch PRs will fight the old hook on every commit.
- **Never run the full migration in the same session that ships this skill.** The skill lands in its own PR; dogfood on the target repo is a separate exercise.

## Gotchas

- **`sf ai agent run` reads from stdin when no positional arg is given** — pipe the plan in, don't try to pass it as a flag.
- **`.cursor/skills/` mirrors `.claude/skills/` exactly**, including nesting. Cursor walks `.cursor/skills/` recursively and treats the leaf folder as the skill identity; category folders are organizational. See `references/cursor-pointer-reference.md`.
- **Numeric prefixes (`0000-`, `9000-`) on monorepo `.cursor/skills/` entries are priority-ordering for that flat layout**, not collision avoidance. Don't preserve them when mirroring a nested layout.
- **`.ai/commands/<area>/` nested subdirs** commonly become namespaced skill prefixes (e.g., `dev-env:foo`). The batch strategy encodes this; don't flatten aggressively.
- **Eval generation is LLM-driven and slow** (~30–60s per skill). A batch of 20 skills takes ~10–20 minutes of `generate-eval` calls. Factor this into each batch PR's expected duration.
- **Cross-batch references**: if skill A references skill B and they land in different batches, B's batch must merge first. Topological ordering of batches is non-optional.

## Out of scope

- **Actual file writes.** Done by the agent in Phase 5, not by this skill.
- **Eval set content.** Delegated to `<sf> ai skills generate-eval` per skill.
- **Anything under `.ai/` other than `commands/` and `context/`.**
- **Fixing failing evals after migration.** That's `author-skill --mode=modify`, not this skill.

## Examples

**Example 1 — dry-run on a small repo.**
User: "migrate .ai to skills, just show me what you'd do"
→ Run Phase 1 audit (20 skills, 3 context files, 2 CLAUDE.md refs).
→ Phase 2: ask layout (user picks (b) flat).
→ Phase 3: render table, user approves.
→ Phase 4: produce plan — 1 deprecated-sweep PR + 2 batch PRs + 1 cleanup PR.
→ Phase 5: skipped because prompt contained `--dry-run`. Report the plan markdown to the user. Done.

**Example 2 — full run on a medium repo (50 skills).**
User: "run sf ai skills migrate"
→ Phase 1: audit finds 50 skills, 5 diverging from existing `.claude/skills/`, 8 CLAUDE.md references, 3 `sf ai rules build` hits.
→ Phase 2: user picks (a) mirror source structure.
→ Phase 3: user reviews proposal, asks about 2 rules, approves.
→ Phase 4: plan — 1 sweep PR + 3 batch PRs (20/20/10 skills) + 1 cleanup PR.
→ Phase 5: pipe plan into `<sf> ai agent run --deep-plan`. Agent produces the stack. Report the 5 PR URLs.
→ Phase 6: (after agent completes) `sf ai skills check` passes. Report delta: 50 new skills discovered, 0 regressions.
