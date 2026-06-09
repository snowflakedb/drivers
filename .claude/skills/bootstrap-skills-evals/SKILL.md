---
name: bootstrap-skills-evals
description: Bootstraps a repo for the `sf ai skills` convention — copies the 5 meta-skills (4 orchestrators + 1 audit skill), adds precommit hooks, generates CI steps, installs telemetry hooks for Claude Code and Cursor, writes the README. Use when the user says "set up sf ai skills", "bootstrap skills", or "run sf ai skills repo-setup".
argument-hint: "[repo-root]"
allowed-tools: [Bash, Read, Glob, Grep]
---

# bootstrap-skills-evals

Plan a repo's one-time onboarding to the `sf ai skills` convention. The bootstrap is all-or-nothing: every target repo gets the same core content — 5 meta-skills (4 lifecycle orchestrators + 1 audit skill), 2 precommit hooks, CI check + eval steps, 2 telemetry hook files (Claude Code + Cursor), a lifecycle README, and a root CLAUDE.md pointer. **This skill does not write files.** It audits, asks a few integration questions, produces a PR plan, and hands it off to `sf ai agent run` which produces the actual PR.

**Scope:** repo-level bootstrap only. Never migrates existing `.ai/commands/` or `.ai/context/` content — that's `migrate-repo-to-skills`. Never generates eval sets for real skills — that's `generate-eval-set`.

## Adaptation principle — read this before Phase 1

This skill's references (`ci-buildkite.md`, `ci-github-actions.md`, `precommit-hooks.md`) show snowdev's implementation as an **example**, not a **prescription**. Every repo has its own CI shape, precommit conventions, change-routing plugins, and tooling provisioning. The references define the **requirements** — what skill_check / skill_eval / the precommit hooks must accomplish, when they must run, what failure semantics they must have. The agent's job is to realize those requirements using the conventions the adopter's repo already uses.

**Before writing any change proposal, audit the repo:**

- How does Buildkite / GitHub Actions organize steps here today? Is there a central steps dir with auto-discovery, a single pipeline file, a plugin-based change-routing convention (`changed_target_discovery`, `monorepo-diff`, custom)? Match it.
- How do existing precommit hooks handle CI-skip, shell conventions, script location? Match them.
- Is `sf` available on the CI runners this repo uses? If not, flag it as a prerequisite the adopter must resolve before the CI steps can land — the wrapper scripts' CI-skip guards will make a missing `sf` look like a passing check.

**Do not introduce snowdev's conventions into a repo that uses different ones.** A verbatim-copied `.buildkite/steps/skill_check.yml` into a repo whose pipeline uses `changed_target_discovery` will produce YAML that looks right and doesn't fire. That's worse than no CI step.

When the repo has no existing pattern to match (greenfield CI, first precommit hook), the snowdev example is a safe starting point — adopt it directly. Otherwise, adapt.

Execute the following steps in order.

## Workflow

### Phase 0 — Resolve `sf` binary

Load `metadata/parameters.md` and follow its instructions. Use the resulting `<sf>` path for every subsequent invocation.

### Phase 1 — Audit

Inventory the target repo. The audit is read-only and produces the facts that feed the proposal phase.

1. **CI system detection.** Look for the three supported flavors:
   - Buildkite — `<repo-root>/.buildkite/`
   - GitHub Actions — `<repo-root>/.github/workflows/`
   - Jenkins — `<repo-root>/Jenkinsfile`, or `Jenkinsfile` files under `<repo-root>/pipelines/`, `<repo-root>/jenkins/`, or `<repo-root>/.jenkins/`. Also detect shared-library-only setups (no Jenkinsfile in this repo but the team uses Jenkins via an upstream library) by checking for `Jenkinsfile.shared`, `jenkins/jobs/`, or asking the user in Phase 2 if the repo has no Jenkinsfile but other signs point at Jenkins.

   Possible outcomes:
   - Exactly one → propose that flavor, confirm in Phase 2.
   - Multiple → propose all detected (some repos have more than one; monorepos especially).
   - Neither/none → ask the user in Phase 2.
   - If the prompt text contains `--ci=buildkite`, `--ci=github-actions`, or `--ci=jenkins`, skip detection and use the forced value.

   If the repo uses a CI system we don't template (CircleCI, Travis, etc.) the audit surfaces it clearly but the skill does not invent a template — user picks one of the three supported flavors (`buildkite` / `github-actions` / `jenkins`) to add alongside, or aborts.

2. **Precommit state.** Read `<repo-root>/.pre-commit-config.yaml` (if present). Note whether `sf-ai-skills-check` or `sf-ai-skills-bridge` hook ids already exist (also check for the deprecated `sf-ai-skills-refresh-pointers` predecessor — flag it as something the user needs to remove manually). Also note whether legacy `sf-ai-rules-build` or `sf-ai-rules-lint` hooks are present — those are `migrate-repo-to-skills`'s concern; flag them for the user's reference, do NOT touch them from this skill.

3. **Existing meta-skills.** Glob `<repo-root>/.claude/skills/*/SKILL.md` and note presence of each of the 5 meta-skills: the 4 orchestrators (`author-skill`, `generate-eval-set`, `run-skill-evals`, `migrate-repo-to-skills`) plus the 1 audit skill (`configure-skill-settings`). For each present, read and diff against snowdev's source at `<snowdev>/.claude/skills/<name>/` (including `metadata/`, `references/`, and `eval_sets/` subtrees where present) — the idempotency matrix in `references/idempotency-rules.md` decides skip / ask. See `references/orchestrator-manifest.md` for the full list and which subdirectories each ships.

4. **Lifecycle README + CLAUDE.md snippet.** Check whether `<repo-root>/.claude/skills/README.md` exists and whether the root `<repo-root>/CLAUDE.md` already mentions `sf ai skills` / `.claude/skills/README.md`.

5. **CI step / stage collisions.** Check for existing artifacts per chosen flavor:
   - **Buildkite**: existing `<repo-root>/.buildkite/steps/skill_check.yml` and `/skill_eval.yml` (or stage entries in the pipeline file).
   - **GH Actions**: existing `<repo-root>/.github/workflows/skill-check.yml` and `/skill-eval.yml`.
   - **Jenkins**: grep the discovered Jenkinsfile(s) for `skill-check` / `skill-eval` stage names. There's no fixed file path — the stages live inside whatever Jenkinsfile the repo already has, so collision detection is grep-based.

   Any collision → ask in Phase 2.

6. **Telemetry hooks state.** Read `<repo-root>/.claude/settings.json` and `<repo-root>/.cursor/hooks.json` if present. Note per-file state for each: missing / exists-no-hooks-key / hooks-key-exists-no-snowdev-entries / snowdev-entries-already-merged. Detection commands and state semantics in `references/telemetry-hooks.md`. The bootstrap MUST NEVER overwrite these files — adopters may have their own `permissions` block (in `.claude/settings.json`) or their own custom hooks; both must be preserved. Phase 3 proposes create-or-merge based on the state captured here.

7. **Legacy `.ai/commands/` and `.ai/context/` detection.** Check whether `<repo-root>/.ai/commands/` or `<repo-root>/.ai/context/` exists with content (one or more `.md` files inside). The bootstrap does NOT migrate this content (that's `migrate-repo-to-skills`'s job — see Out of Scope), but it MUST record the finding with per-directory file counts. Bootstrap proceeds normally either way; legacy presence does NOT block or alter the bootstrap itself. The finding is consumed at the end of the workflow by the **What's next** output, which offers migration conversationally to the user (see Output format). Rationale: migrate-repo-to-skills is one of the 5 meta-skills the bootstrap installs, so by the time bootstrap finishes the migrate skill is already in `.claude/skills/` and Claude can auto-load it via path B if the user signs off. Doing the order this way means the user stays in a single Claude session for both bootstrap and migration.

Record everything. Present a concise audit summary to the user at the end of this phase (short bulleted list, not the raw data dump).

### Phase 2 — Integration questions (blocking)

Bootstrap content is NOT negotiable. Do not ask "which skills?" or "do you want CI?" — the answer is always "all five meta-skills, yes CI, always". What the user decides is *how to integrate* with their existing setup.

1. **CI flavor** (only if not forced by `--ci=...`). Three supported flavors: `buildkite`, `github-actions`, `jenkins`.
   - One detected → "Detected `<flavor>`. Generate `<flavor>` CI?" (default yes; `none` is not a valid answer).
   - Multiple detected → "Detected `<flavor-a>` and `<flavor-b>`. Generate for all detected?" (default yes; the user can pick a subset if they prefer).
   - None detected → "No CI system detected. Which would you like? `buildkite`, `github-actions`, or `jenkins`."
   - Unsupported CI detected (CircleCI, Travis, etc.) → "Detected `<x>` (unsupported). Would you like to add `buildkite`, `github-actions`, or `jenkins` alongside, or abort?"

2. **CLAUDE.md snippet placement.** Show the user the first ~30 lines of their root CLAUDE.md and propose an insertion point (after the intro paragraph, before the first H2 section, is usually right). If the file has no obvious structure or the proposed spot isn't appropriate, ask.

3. **Conflict resolution** — for each artifact the audit found already present with diverged content, show existing vs proposed and ask: overwrite / keep existing / merge. Never silent-overwrite.

Block on explicit confirmation before moving to Phase 3.

### Phase 3 — Propose

Render the full bootstrap manifest showing every file to create or modify with final content. The integration choices from Phase 2 are already baked into the proposal — the user is approving the final artifact list, not the shape of the bootstrap.

Group the manifest by category. For each category below, the referenced `references/*.md` file defines **requirements** (what the final artifact must do) and shows snowdev as an **example**; the agent must read the reference, audit the target repo's existing conventions, and synthesize content that realizes the requirements using those conventions. See the "Adaptation principle" section above — verbatim-copying references that don't match the repo produces artifacts that look right and don't work.

1. **Meta-skills** — 5 entries total (4 orchestrator skills + 1 audit skill — `configure-skill-settings`), each with action (copy / skip-identical / overwrite-after-confirmation). These ARE copied verbatim from snowdev — meta-skills are the shared spec, not adaptable content. The 4 orchestrators do not ship `eval_sets/` (exempt from routing-accuracy evals because they wrap CLI commands); `configure-skill-settings` DOES ship with an `eval_sets/routing-accuracy.yaml` — copy it. See `references/orchestrator-manifest.md` for the full list, source paths, and target layout.

2. **Precommit hooks** — 2 hook entries in `.pre-commit-config.yaml` plus 2 wrapper scripts under `.pre-commit-scripts/`. Requirements and snowdev example in `references/precommit-hooks.md`. Before composing the final hook YAML and wrapper scripts:
   - Inspect the repo's existing `.pre-commit-config.yaml` structure (single `local` block, multiple blocks grouped by topic, external `- repo:` references, `meta` hooks).
   - Inspect the repo's existing `.pre-commit-scripts/**` to learn its wrapper-script conventions (shebang, shell strictness, CI-skip env vars, file naming).
   - Compose wrapper scripts that match those conventions while satisfying the requirements in the reference.

3. **CI steps** — 2 stages/files per chosen flavor. Buildkite requirements in `references/ci-buildkite.md`; GH Actions requirements in `references/ci-github-actions.md`; Jenkins requirements + `withSfCli` provisioning guidance in `references/ci-jenkins.md`. Before composing the final YAML (or Jenkinsfile stages):
   - Find where this repo's pipelines/workflows actually live (paths differ per repo — single file, per-pipeline subdirs, auto-discovery).
   - Identify the repo's change-routing convention (native `if_changed:`, plugin-based matchers, path filters in `on.pull_request.paths`, custom plugins like `changed_target_discovery`).
   - Identify the repo's install-tooling pattern for other CLI binaries in CI, and match it for `sf-cli`.
   - Compose steps that realize the requirements table from the reference using this repo's conventions.
   - The `skill-eval` stage today calls a repo-specific eval driver script — NOT `sf ai skills eval --context=ci` directly. The CLI shells out to `sf ai claude` which isn't provisioned on CI workers, so a direct CLI call returns parse_error on every prompt and reports FAIL on routing-correct skills. Each ci-flavor reference's "Why a script, not `sf ai skills eval`?" section explains the cause and links an example. Surface this to the user when proposing — don't generate a step that calls `sf ai skills eval --context=ci` directly.

4. **Lifecycle README** — one file at `.claude/skills/README.md`. Read snowdev's canonical `.claude/skills/README.md` at runtime and include its full content verbatim in the manifest. Do NOT edit, summarize, or customize for the target repo — the convention is identical across repos. See `references/lifecycle-readme-template.md` for rationale.

5. **CLAUDE.md snippet** — one insertion in the root CLAUDE.md. Content in `references/claude-md-snippet.md`.

6. **Telemetry hooks** — 2 entries: the `hooks` block of `<repo-root>/.claude/settings.json` and the entire `<repo-root>/.cursor/hooks.json` file. Snowdev's `.claude/settings.json` (the `hooks` key only — NOT the `permissions` key) and `.cursor/hooks.json` are the canonical sources, read at runtime from snowdev main. These ARE verbatim like the meta-skills and the lifecycle README — the hook commands are the shared spec, not adaptable. What IS adopter-specific is the file shape around them: existing `permissions`, other hooks, custom keys MUST be preserved. Requirements, detection states, and merge semantics in `references/telemetry-hooks.md`. Surface a callout in the proposal if the adopter's `.claude/settings.json` already has a `permissions.allow` block — they may need to add `Bash(sf:*)` manually so the hook commands can execute.

**Present the proposal with each agent-composed artifact labelled as such.** The user needs to see what was snowdev-verbatim (orchestrators, README, telemetry hook commands) vs what the agent synthesized for this repo (hook YAML, wrapper scripts, CI steps) — the synthesized pieces deserve closer review because they encode the adaptation to the target repo.

Ask: "Approve bootstrap? y/n/edit". If `edit`, iterate — re-present the affected section and re-ask. Never proceed without explicit approval.

### Phase 4 — Plan

Produce a PR plan as a single markdown document. This is a one-shot bootstrap, NOT batched across multiple PRs — the entire bootstrap ships in one PR. Sections of the plan:

- Branch name suggestion (e.g., `bootstrap-sf-ai-skills` or `<user>-bootstrap-skills-evals`).
- Commit message / PR title.
- Per-file content (create / modify), with the full final text inlined.
- Expected `sf ai skills check` finding delta after the PR merges.
- Post-merge verification steps.

Write the plan to stdout so the user can review it verbatim before handoff.

### Phase 5 — Handoff

If the prompt text contains `--dry-run`, stop here and report the plan path. Do NOT proceed to the agent handoff.

Otherwise, pipe the plan to `sf ai agent run`:

```
echo "<plan>" | <sf> ai agent run --deep-plan
```

Capture and report the agent's output (PR URL / branch name). The agent may background — don't block waiting for completion.

### Phase 6 — Verify (post-agent)

After the agent's PR merges, run `<sf> ai skills check <repo-root>` and compare to the pre-bootstrap baseline captured in Phase 1. Report:

- `check` should report one `coverage.missing_recommended` finding per newly-installed orchestrator skill — the 4 orchestrators are exempt from routing-accuracy evals by design (they wrap LLM-driven CLI commands rather than being model-routed), so this warning is expected for each. `configure-skill-settings` should NOT produce this finding because it ships with an `eval_sets/routing-accuracy.yaml` — if it does, the copy didn't include the eval_sets/ subdir; re-run Phase 3 for it.
- No `frontmatter.name.missing` findings should appear on the installed meta-skills — the snowdev sources ship with `name:` already populated. If one does, the copy didn't include the full frontmatter block; re-run Phase 3 for that skill.
- Any OTHER new finding type is unexpected — investigate before merging next PR.
- `sf ai skills bridge` should be a no-op (the precommit hook already ran).
- **Telemetry hooks** — confirm `.claude/settings.json` contains snowdev's `sf ai __hook --ide claude` entries for both `user-prompt-submit` and `post-tool-use` events, and `.cursor/hooks.json` contains the `before-submit-prompt` entry. See `references/telemetry-hooks.md` for the exact command strings (snowdev-canonical) and a "what these hooks do at runtime" walkthrough. Also run `which sf` to confirm `sf` is on the developer's PATH; without it, the hooks return non-zero and telemetry is silently lost (the prompt continues anyway by design — but the signal is gone).

## Output format

Always end with:

1. **Phase reached** — 1/2/3/4/5/6.
2. **Audit summary** — CI system detected, orchestrators already present, existing artifacts in the way.
3. **Integration decisions** — CI flavor picked, CLAUDE.md snippet location.
4. **Plan path** (Phase 4+) — where the bootstrap plan markdown lives.
5. **PR URL** (Phase 5+) — output from `sf ai agent run`.
6. **Final check delta** (Phase 6+) — diff against pre-bootstrap baseline.
7. **What's next** — name the immediate next user action so the user knows where to pick up in a future session.

   **If Phase 1 step 6 detected legacy `.ai/commands/` or `.ai/context/` content**, lead with a conversational migration nudge that gives the user enough context to make an informed decision and an explicit yes/no signoff. Use roughly this shape (adapt the file counts and PR URL to the actual run):

   > Bootstrap complete. PR `<url>` opens to install meta-skills, hooks, and CI.
   >
   > I noticed this repo has legacy `.ai/commands/` (`<N>` files) and/or `.ai/context/` (`<M>` files) that need to be ported to the new convention. Migrating these is the natural next step — it produces a separate PR stack that ports the legacy content to `.claude/skills/` and `.claude/rules/`.
   >
   > The `migrate-repo-to-skills` skill is now installed in this repo. I can auto-load it and walk you through the migration in this session — Phase 1 audits your `.ai/` content, Phase 2 asks you to pick a target layout (mirror / flat / custom), Phase 3 reviews the proposed mapping, then it hands off to produce the PR stack.
   >
   > Want me to start the migration now? (yes / no — happy to wait if you'd rather review the bootstrap PR first or do this in a fresh session.)

   Block on the answer. If `yes`, auto-load `migrate-repo-to-skills` in the same session and let it run from Phase 0. If `no`, tell the user that migration can be triggered later by typing "migrate .ai to skills" in any Claude session in this repo (path B), or by running `sf ai skills migrate` from a terminal.

   **If no legacy content was detected**, use the simpler closer:
   - *After the bootstrap PR merges, re-run `sf ai skills check` and `sf ai skills status` to confirm the new hooks/CI steps/README are wired.*
   - *Then run `sf ai skills status` to see which skills lack eval sets and fill those gaps with `sf ai skills generate-eval <skill-dir>`.*

## Quality rules

- **Never negotiate which artifacts get installed.** Bootstrap content is fixed: 5 meta-skills (4 orchestrators + 1 audit skill), 2 precommit hooks, CI check + eval, 2 telemetry hook files (Claude + Cursor), README, CLAUDE.md snippet. Don't let the user opt out of any piece — a half-bootstrap is a misconfiguration.
- **Telemetry hooks: never overwrite, always merge.** The hook command strings are spec (`sf ai __hook --ide <ide> --hook-type <type>`); the file shape around them (adopter `permissions`, other custom hooks, top-level keys) MUST be preserved. See `references/telemetry-hooks.md` for merge semantics.
- **Never skip the CI integration question** (Phase 2). CI is mandatory; `--ci=none` is not supported.
- **Always detect-then-skip for idempotency.** Bytes-identical artifacts are "already installed, skipping". Diverged artifacts require explicit user choice.
- **Never touch `.ai/commands/` or `.ai/context/`.** Migration is a separate concern.
- **When legacy `.ai/` is detected, MUST end with the migration prompt and explicit yes/no signoff.** Phase 1 step 6 detection feeds the Output format's "What's next" — the bootstrap is not complete until the user has explicitly answered the migration yes/no. Never auto-load `migrate-repo-to-skills` without the user's explicit `yes`. Never finish silently when legacy content was detected — that leaves the user without the next-step guidance they need.
- **Never touch `.ai/review/`, `.ai/casper-tasks/`, `.ai/mcp/`, `.ai/plans/`, `.ai/README.md`, or `.ai/OWNERS.yml`** — anything under `.ai/` beyond commands/context is out of scope for every `sf ai skills` command.
- **Never run `sf ai agent run` in `--dry-run` mode.** `--dry-run` means stop at Phase 4.
- **Never hand-write files under `.agents/skills/`.** The `bridge` precommit hook creates the symlinks. Committing real files there makes the bridge fail (or requires `--lenient`).

## Gotchas

- **`sf ai agent run` reads from stdin when no positional arg is given** — pipe the plan, don't pass as a flag.
- **snowdev's `.claude/skills/<meta-skill>/` trees are the canonical source** for all 5 meta-skills (4 orchestrators + `configure-skill-settings`). Read them at runtime from wherever snowdev is checked out. Do not embed their content in this skill's references — they'd drift. See `references/orchestrator-manifest.md` for the full list.
- **Repos outside Snowflake infra using Buildkite need no plugin** — our generated step YAML uses plain `command: sf ai skills ...` instead of the internal `${GLOBAL_PLUGIN}/cmd_runner` that the monorepo uses.
- **Assumes `sf` is on the CI runner's PATH.** If the user's CI image doesn't have sf-cli, the proposal should include an install step pointing to `dev-env/sf-cli/README.md` (the skill can't guess the right install path — ask).
- **GitHub Actions skill-check workflow blocks the PR; skill-eval is advisory.** This matches the contract's `enforcement.eval.ci: advisory` default. Don't flip it without a contract-level decision.
- **`skill-eval` in CI calls a repo-specific driver script, not `sf ai skills eval --context=ci` directly.** The CLI shells out to `sf ai claude` which isn't provisioned on CI workers today (Cortex auth lives on cloud workspaces). A direct CLI call returns parse_error on every prompt. See each ci-flavor reference's "Why a script, not `sf ai skills eval`?" section for cause and example. The CLI works fine for local use on a cloud workspace.
- **Legacy `sf ai rules build` / `sf ai rules lint` hooks** are `migrate-repo-to-skills`'s concern. Flag their presence for the user, don't touch them from this skill.

## Out of scope

- **Migrating existing `.ai/commands/` content.** Use `migrate-repo-to-skills`.
- **Authoring new skills.** Use `author-skill`.
- **Generating eval sets for existing skills.** Use `generate-eval-set`.
- **Running evals.** Use `run-skill-evals`.
- **Jenkins, CircleCI, Travis CI templates.** Only Buildkite and GitHub Actions are supported.
- **`.claude/rules/*.md` scaffolding.** Rules are a migration-time decision, not a bootstrap concern.

## Examples

**Example 1 — greenfield repo, no CI detected.**
User: "bootstrap sf ai skills in this repo"
→ Phase 1: audit finds no `.buildkite/`, no `.github/workflows/`, no `.claude/skills/`, empty CLAUDE.md.
→ Phase 2: ask CI flavor. User picks `github-actions`. Confirm CLAUDE.md snippet placement (only one reasonable location).
→ Phase 3: render proposal. User approves.
→ Phase 4: write plan to stdout.
→ Phase 5: pipe to `sf ai agent run --deep-plan`. Report PR URL.
→ Phase 6: after merge, `sf ai skills check` reports expected conventional findings: `coverage.missing_recommended` × 4 (one per orchestrator; `configure-skill-settings` should NOT appear here since it ships with an eval set). Clean.

**Example 2 — repo already has some orchestrators.**
User: "run sf ai skills repo-setup"
→ Phase 1: audit finds `.buildkite/`, existing `.claude/skills/author-skill/` (bytes-identical to snowdev's source), existing `.claude/skills/run-skill-evals/` (diverged from snowdev's source).
→ Phase 2: confirm Buildkite (detected). Ask about the diverged `run-skill-evals` — user picks "keep existing".
→ Phase 3: proposal excludes the existing orchestrators; proposes only `generate-eval-set` and `migrate-repo-to-skills` as new copies. User approves.
→ Phase 4-5: handoff as normal. PR is smaller since 2 skills were skipped.

**Example 3 — dry-run preview.**
User: "preview the bootstrap without creating a PR"
→ `sf ai skills repo-setup --dry-run`
→ Phases 1-4 run normally. Phase 5 skipped. User sees the full plan in their terminal. No agent is invoked.
