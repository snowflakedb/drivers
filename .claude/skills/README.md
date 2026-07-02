# sf ai skills — Skills & Evals Lifecycle

This repo follows the `sf ai skills` convention for authoring,
validating, and measuring the routing accuracy of Claude Code
skills. Skills live at `.claude/skills/<name>/SKILL.md`. Their
routing-accuracy eval sets live at
`.claude/skills/<name>/eval_sets/routing-accuracy.yaml`. A contract
(see `sf ai skills contract`) defines verdict thresholds,
enforcement levels, and exemption rules that apply cross-repo.

This README is the single source of truth for the lifecycle.
It is deliberately generic and identical across every repo that
adopts the convention — the `sf ai skills repo-setup` command
installs this exact file. **Per-command flag details are not
duplicated here: run `sf ai skills <command> --help` for the
canonical flag reference, and `sf ai skills contract` for the
active contract values.**

## Audience

This README is written for both humans and coding agents.
**Autonomous agents** should treat it as a checklist: read top-to-
bottom, pick the right command for the current state, and execute.
Every command that requires a human-signoff gate is explicitly
marked with **AGENT: block here** — at those points, stop and wait
for the user. Do not guess what the user would approve.

## What this repo ships

When a repo adopts the convention (via `sf ai skills repo-setup`),
it gets all of the following. If any piece is missing, the setup
is incomplete — run `repo-setup` again.

- **4 lifecycle orchestrator skills** under `.claude/skills/`:
  `author-skill`, `generate-eval-set`, `run-skill-evals`,
  `migrate-repo-to-skills`. They auto-trigger from natural-language
  phrases and wrap the CLI so you can say "create a skill" instead
  of memorizing `sf ai skills author`.
- **1 audit skill** under `.claude/skills/`:
  `configure-skill-settings` — repo-hygiene tool teams invoke
  (typically post-migration, or periodically as the skill count
  grows) to audit SKILL.md frontmatter across their `.claude/skills/`
  directories. Does not wrap a CLI command.
- **2 precommit hooks**: `sf-ai-skills-check` (validates every
  skill on commit) and `sf-ai-skills-bridge` (keeps
  `.agents/skills/` populated with symlinks into `.claude/skills/`
  so Cursor and Codex CLI can discover every nested skill). Each
  hook ships with a wrapper script under `.pre-commit-scripts/`
  that gracefully skips on CI runners without sf-cli installed.
- **2 CI steps** under `.buildkite/steps/`, `.github/workflows/`,
  or as stages in an existing `Jenkinsfile` (depending on which
  flavor was chosen at `repo-setup` time): `skill_check` (blocking
  deterministic validation) and `skill_eval` (advisory routing-
  accuracy eval).
- **2 telemetry hook files**: `.claude/settings.json` contains a
  `hooks` block wiring `sf ai __hook` into Claude Code's
  `UserPromptSubmit` and `PostToolUse` events; `.cursor/hooks.json`
  wires the same into Cursor's `beforeSubmitPrompt`. Both are
  required for adoption metrics and routing-accuracy telemetry.
  Adopter-existing entries in either file are preserved — bootstrap
  merges, never overwrites.
- **This README** at `.claude/skills/README.md`.
- **A `## Skills & Evals` pointer** added to the repo's root
  `CLAUDE.md`.

## The contract

There is a contract for the Skills<>Evals convention:
verdict thresholds (PASS / WARN / FAIL), per-eval-type
prompt caps, enforcement levels (advisory / blocking), and
frontmatter exemption rules all live in the contract and apply
cross-repo. **Run `sf ai skills contract` to see the current
values.** Don't duplicate them in repo docs — they may change.
Don't override them locally — contract changes are a cross-repo
decision.

## Lifecycle at a glance

Pick the path that matches the repo's current state:

| Repo state | Start with |
| --- | --- |
| Legacy `.ai/commands/` + `.ai/context/` (no `.claude/skills/`) | **`sf ai skills repo-setup`** → installs meta-skills, hooks, and CI in one PR. When it finishes, it detects the legacy `.ai/` content and offers to auto-load `migrate-repo-to-skills` in the same Claude session — say yes and migration runs next. |
| Greenfield — no `.claude/skills/` and no legacy `.ai/` | **`sf ai skills repo-setup`** → one bootstrap PR. |
| Has `.claude/skills/` + tooling already | **`sf ai skills check` + `status`** for a baseline, then iterate. |
| Daily development on skills | **`author` / `generate-eval` / `eval`** as needed. |
| Before every commit | **Precommit hooks run automatically** (check + bridge). |

## Commands

The CLI surfaces one command per verb. `--help` is canonical for
each command's flags.

### `sf ai skills check`

Deterministic validation against the contract. No LLM calls; safe
to run on every commit and in CI. Surfaces findings at
error / warning / info severity.

- **When to use**: any time. Default diagnostic step for "what
  state is this repo in?"
- **How to act on output**: errors block (must fix); warnings are
  advisory (should fix); info is informational.
- **Precommit**: already wired via `sf-ai-skills-check` hook.
  Fails the commit on contract errors.

### `sf ai skills status`

Scorecard view — per-eval-type coverage, missing eval sets, repo
health summary. JSON when piped, pretty table on TTY. No LLM.

- **When to use**: to understand "how healthy is this repo's
  skills posture?" Good companion to `check`: `check` tells you
  *what's wrong*, `status` tells you *what's missing*.

### `sf ai skills author [skill-dir] [--mode=create|modify]`

LLM-driven authoring of a new SKILL.md or a surgical patch to an
existing one. Mode auto-detects from whether a SKILL.md exists.

- **When to use CREATE**: adding a brand-new skill.
- **When to use MODIFY**: fixing a trigger gap, tightening a
  description, or applying review feedback on an existing skill.
- **AGENT: block here.** After the command returns successfully,
  review the written SKILL.md and only commit once the user (or
  a human reviewer) has approved the change. Do NOT auto-commit.

### `sf ai skills generate-eval <skill-dir>`

Asks Claude to author a routing-accuracy eval set for a skill.
GENERATE mode authors fresh YAML; UPDATE mode preserves
team-curated prompts and suggests only warranted changes.
Auto-detects mode based on whether an eval set exists.

- **When to use**: every non-exempt skill needs an eval set
  (skills with `disable-model-invocation: true` are contractually
  exempt).
- **AGENT: block here.** After the command writes the YAML,
  review the generated prompts. Agents running autonomously
  should not commit LLM-authored eval sets without a signoff.

### `sf ai skills eval [path]`

Runs routing-accuracy evals against Claude for the given skill(s)
and reports PASS / WARN / FAIL. Output auto-detects: pretty table
on TTY, JSON when piped.

- **When to use**: verify routing after authoring a skill; gate
  release on eval health.
- **How to interpret verdicts**:
  - **PASS** — routing-accuracy at or above the contract's PASS
    threshold.
  - **WARN** — between WARN and PASS thresholds. Advisory.
  - **FAIL** — below the WARN threshold. Fix via
    `author --mode=modify --feedback "..."` and re-run.
  - **EXEMPT** — skill has `disable-model-invocation: true`;
    contractually not routing-evaluated.
  - **SKIPPED** — skill has no eval set. Run `generate-eval`.
- **Failed prompts** surface Claude's selected skill + reasoning
  so you can debug without re-running each prompt manually.

### `sf ai skills migrate [repo-root]`

LLM-driven orchestrator. Plans a repo's migration from the legacy
`.ai/commands/` + `.ai/context/` layout to `.claude/skills/` and
`.claude/rules/`. Hands the plan to `sf ai agent run` which
produces the actual PR stack.

- **When to use**: one-time, on repos that still have the legacy
  `.ai/commands/` layout. If `.claude/skills/` already exists and
  `.ai/commands/` doesn't, skip this command.
- **AGENT: block here.** The skill has explicit signoff gates at
  Phase 2 (layout choice: mirror / flat / custom) and Phase 3
  (per-artifact mapping approval). These are repo-altering
  decisions. Block and wait; do not guess.
- **Use `--dry-run`** to produce the plan without handing off to
  the agent pipeline — useful for reviewing what would happen.

### `sf ai skills repo-setup [repo-root]`

LLM-driven orchestrator. One-time bootstrap: installs the 4
orchestrator skills, the 2 precommit hooks, the 2 CI steps, this
README, and the CLAUDE.md pointer.

- **When to use**: once per repo, after `migrate` (if the repo had
  a legacy `.ai/` layout). Safe to re-run — the skill is
  idempotent and detects already-installed pieces.
- **AGENT: block here.** The skill blocks at Phase 2 (CI flavor
  choice when ambiguous, placement of the CLAUDE.md snippet) and
  Phase 3 (full manifest approval). Block and wait.
- **Use `--dry-run`** to see the manifest without producing a PR.

### `sf ai skills bridge [path] [--lenient]`

Deterministic Go command. Walks `.claude/skills/` and creates a
git-tracked symlink at every matching
`<module>/.agents/skills/<skill>` pointing at the sibling
`.claude/skills/<skill>` directory. One bridge populates both
Cursor's and Codex CLI's native discovery paths (both walk
`.agents/skills/` recursively and both explicitly support
symlinks). Deletes orphan symlinks whose source is gone.

- **When to use**: automatically, via the `sf-ai-skills-bridge`
  precommit hook. Manual invocation is fine if you want to refresh
  bridges immediately.
- **Behavior**: exits non-zero if it created, retargeted, or
  deleted anything. Precommit wrapper fails the commit and asks
  you to re-stage — same pattern as `gofmt` precommit.
- **`--lenient`**: tolerate existing non-symlink content at
  `.agents/skills/` bridge paths (e.g. deliberate full-copy trees
  like snowdev's `.agents/skills/snowci/**`). Missing bridges are
  still created; non-symlink entries are logged as `SKIP` and left
  alone. Precommit hooks can opt in via
  `SF_AI_SKILLS_BRIDGE_LENIENT=1` in the environment.
- **Don't hand-edit** `.agents/skills/` entries — the next
  `bridge` will either overwrite (symlinks) or error (non-symlinks,
  unless `--lenient`).
- **`.claude/rules/` is NOT bridged** — neither Cursor nor Codex
  discovers rules. Rules remain Claude-only.

### `sf ai skills contract`

Prints the active contract as JSON or YAML. Use when in doubt
about thresholds, enforcement levels, or exemption rules.

- **When to use**: debugging a verdict ("why did this skill get
  WARN?"), understanding a finding ("what does
  `coverage.missing_recommended` mean?"), or verifying the
  contract version.

## The 5 meta-skills

These live at `.claude/skills/<name>/`. Two groups:

### 4 lifecycle orchestrators (CLI-command wrappers)

**The only reason** users get to type "create a skill" instead of
`sf ai skills author`. They auto-trigger from natural-language
phrases so the CLI commands are reachable without memorization.

| Skill | Auto-triggers on | Wraps |
| --- | --- | --- |
| `author-skill` | "create a skill", "new skill", "write a skill", "fix this skill" | `sf ai skills author` |
| `generate-eval-set` | "generate evals", "add eval set", "write evals for my skill" | `sf ai skills generate-eval` |
| `run-skill-evals` | "run skill evals", "eval my skills", "test skill routing" | `sf ai skills eval` |
| `migrate-repo-to-skills` | "migrate .ai to skills", "convert .ai/commands" | `sf ai skills migrate` |

Each orchestrator has its own SKILL.md body with the full phase
sequence and signoff gates. This README summarizes; read the skill
body when invoking.

### 1 audit skill (repo-hygiene tool)

Does NOT wrap a CLI command. Teams invoke it when they want to
audit frontmatter health across their growing skill set — typically
post-migration, or periodically as the skill count grows.

| Skill | Auto-triggers on | What it does |
| --- | --- | --- |
| `configure-skill-settings` | "audit my skill settings", "review skill frontmatter", "configure skill settings" | Audits SKILL.md frontmatter across `.claude/skills/` — flags `disable-model-invocation`, paths vs globs, description length, name/directory mismatches, skill visibility/shadowing |

Ships with an eval set (`eval_sets/routing-accuracy.yaml`) because
its trigger surface IS routing-accuracy-evaluated — unlike the
orchestrators, which are exempt because they wrap CLI commands
rather than being model-routed at the decision level.

## CI integration

Two CI steps ship per repo, in whichever flavor your repo uses:

| Flavor | `skill_check` location | `skill_eval` location |
|---|---|---|
| Buildkite | `.buildkite/steps/skill_check.yml` | `.buildkite/steps/skill_eval.yml` |
| GitHub Actions | `.github/workflows/skill-check.yml` | `.github/workflows/skill-eval.yml` |
| Jenkins | `skill-check` stage in the existing `Jenkinsfile` | `skill-eval` stage in the existing `Jenkinsfile` |

- **`skill_check`** runs `sf ai skills check --context=ci`.
  Blocks the PR on contract errors.
- **`skill_eval`** runs the repo's eval driver script on changed
  skills. Advisory per the contract's default enforcement. See
  the per-flavor reference (`bootstrap-skills-evals/references/ci-<flavor>.md`)
  for the rationale on the driver-script pattern and an example
  to adapt.

For Jenkins repos, the worker needs `sf` provisioned — Jenkins
agents don't ship with `sf` the way Buildkite/GHA runners can.
Snowflake-internal Jenkins repos use the `withSfCli` helper from
`snowflake-eng/jenkins_utils`'s `pipeline-utils` shared library
(downloads `sf`, sets cert + env vars, runs the closure, cleans
up). External adopters bake `sf` into the worker image or install
it at stage-start. See
`bootstrap-skills-evals/references/ci-jenkins.md` for the full
pattern.

CI enforcement levels are contract-driven, not per-repo. Changes
to what blocks vs. advises require a contract update, not a
workflow edit.

## Precommit integration

Two hooks via `.pre-commit-config.yaml`:

- **`sf-ai-skills-check`** runs
  `sf ai skills check --context=precommit --changed-only --severity=error`
  on changes under `.claude/`. Fails the commit on contract errors.
  `--context=precommit` is the important bit: it teaches
  `--changed-only` to include staged-but-uncommitted files, so the
  first commit that introduces a new skill is validated before it
  lands (same contract as gofmt / golangci-lint — validate the
  index, not mid-edit worktree noise).
- **`sf-ai-skills-bridge`** runs `sf ai skills bridge`. Fails if
  it created, retargeted, or deleted any symlink so you re-stage
  the regenerated `.agents/skills/` entries.

Both hooks have CI-runner skip guards — they skip gracefully on
runners without sf-cli installed (detected via `CI`, `JENKINS_URL`,
or `BUILDKITE` env vars). Local commits must have sf-cli on PATH.

## Agent workflow — end-to-end

Linear checklist an autonomous agent can follow from zero on an
unknown repo. Always honor the signoff gates.

**This is a multi-session journey, not a single chat.** Onboarding a
repo typically takes days, not minutes — each phase produces one or
more PRs that must merge before the next phase can start. Expect
the user to close the session between steps, merge PRs on a normal
review cadence, then return for the next step. At the start of
every session, re-read this checklist and figure out where the repo
is by inspecting filesystem state (does `.ai/commands/` still
exist? does `.claude/skills/README.md` exist? does `status` show
gaps?).

1. **Baseline**. Run `sf ai skills check` and `sf ai skills status`
   to understand the repo's current state. Capture the finding
   count + scorecard as your "before" snapshot.

2. **Bootstrap** (only if `.claude/skills/README.md` does not exist
   at the repo root, OR any of the 5 meta-skills — 4 orchestrators
   or the `configure-skill-settings` audit skill — is missing):
   - Run `sf ai skills repo-setup`.
   - Block at Phase 2 (CI flavor, CLAUDE.md placement) and Phase 3
     (manifest approval). Wait for the user.
   - Produces one bootstrap PR.
   - **If the repo has legacy `.ai/commands/` or `.ai/context/`
     content**, the bootstrap's "What's next" surfaces that as a
     conversational migration nudge with an explicit yes/no
     signoff. If the user says yes, the agent auto-loads
     `migrate-repo-to-skills` in the same session — proceed with
     step 3 immediately. If no, step 3 still runs eventually
     (whenever the user is ready); the meta-skill is now installed
     and triggerable via `sf ai skills migrate` or path B.
   - **After the bootstrap PR merges**, return to this checklist
     and continue with step 3 (if applicable) or step 4.

3. **Legacy migration** (only if `.ai/commands/` or `.ai/context/`
   exists at the repo root — this step runs AFTER bootstrap so the
   `migrate-repo-to-skills` skill is locally installed):
   - Either auto-loaded inline by step 2's migration nudge (user
     said yes), or invoked later via `sf ai skills migrate` (CLI)
     or "migrate .ai to skills" (path B in any session).
   - Block at Phase 2 (layout choice) and Phase 3 (proposal
     approval). Wait for the user. Do not guess.
   - **Migration produces a PR stack**, not a single PR: one
     deprecated-tooling sweep PR, followed by N batch PRs (one per
     logical group of 15–20 skills), followed by one cleanup PR.
     Each PR must merge in topological order before the next opens
     — agents that touch skill A before skill A's containing batch
     has merged will land dangling references. Expect this phase to
     take days across multiple sessions.
   - **After the full migration stack merges**, return to this
     checklist and continue with step 4.

4. **Fill gaps**:
   - Re-run `sf ai skills check` and `sf ai skills status` to see
     the post-migration/post-bootstrap state; capture the fresh
     finding counts as your starting point for gap-fill.
   - For each skill without an eval set (surfaced by `status`):
     run `sf ai skills generate-eval <skill-dir>`.
     Block for a user signoff on the generated YAML.
   - For each skill flagged by `check` at error severity:
     run `sf ai skills author --mode=modify <skill-dir> --feedback
     "<fix>"`.  Block for signoff.
   - PR shape for gap-fill depends on the repo's review culture —
     ask the user whether to batch fixes into one PR or split by
     skill. Don't assume.

5. **Verify**:
   - Run `sf ai skills eval` on the full repo, or
     `sf ai skills eval <skill-dir>` for a targeted run.
   - For FAIL / WARN verdicts, iterate on the skill via
     `author --mode=modify` and re-eval.

6. **Ship**:
   - Commits pass precommit hooks automatically.
   - CI gates `skill_check` (blocking) and `skill_eval`
     (advisory).
   - No special shipping step beyond the usual repo workflow.

At every step: if a command has a signoff gate, **block and wait**.
The gates exist because these are repo-altering decisions humans
need to approve.

At every step **transition**: if the previous step produced a PR
(or stack), confirm it merged before moving on. Running step N+1
while step N's PRs are still open will produce conflicting work.

## Troubleshooting

A short list of things `--help` won't tell you.

1. **"eval reports EXEMPT for my skill"** → The skill has
   `disable-model-invocation: true` in its frontmatter.
   Contractually exempt from routing-accuracy (the model never
   sees it at routing time). Not a failure — no action needed.

2. **"My skill isn't auto-triggering on a phrase a user typed"**
   → The description is missing that exact phrase. Skill routing
   is driven by the description's trigger phrases. Fix via:
   `sf ai skills author --mode=modify <skill-dir> --feedback "add
   'the exact phrase' verbatim to the description"`.

3. **"`check` passes locally but fails in CI"** → CI runs with
   `--context=ci`, which may apply stricter enforcement per the
   contract. Reproduce locally with
   `sf ai skills check --context=ci`.

4. **"`bridge` precommit keeps failing my commit"** → Either
   (a) bridges are out of sync (missing / wrong target / orphan) —
   the hook auto-fixes them; stage the regenerated
   `.agents/skills/` entries and re-commit (same pattern as
   `gofmt` precommit), or (b) the repo has deliberate full-copy
   directories at `.agents/skills/` and strict mode is rejecting
   them — set `SF_AI_SKILLS_BRIDGE_LENIENT=1` in your shell or
   precommit environment to tolerate non-symlink entries.

5. **"An eval verdict changed between runs on the same skill"**
   → LLM non-determinism within the contract's threshold
   tolerances. The thresholds are built for this — a skill that
   passes 88% of prompts one run and 85% the next is still
   well above a typical 80% PASS threshold. Only treat verdict
   flips as real signal; don't re-run looking for a different
   answer.

## Planned / coming soon

- **`sf ai skills doctor`** — one-shot diagnose-and-fix for common
  skill health issues. **No ETA — adopt today using `check` +
  `status` manually; `doctor` will be a convenience wrapper around
  the same primitives when it ships.**

## References

- **`sf ai skills <command> --help`** — canonical flag reference.
  Always prefer this over any snapshot in prose.
- **`sf ai skills contract`** — canonical contract values.
  Thresholds, enforcement levels, exemptions.
- **Individual orchestrator skills** at
  `.claude/skills/<name>/SKILL.md` — canonical phase sequences
  and signoff gates.
