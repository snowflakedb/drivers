# Buildkite CI integration — requirements, example, adapt

This reference tells the bootstrap proposal how to add two CI steps
(`skill_check` and `skill_eval`) to a Buildkite-using repo. It is
deliberately **not** a paste-this-YAML template. Every Buildkite
repo organizes pipelines differently; dropping snowdev-shaped YAML
into a repo that uses a different convention produces steps that
look right and don't fire. The bootstrap's job is to realize the
requirements below using whichever convention the adopter's repo
already uses.

## Requirements (non-negotiable)

Two steps must end up in the adopter's Buildkite pipeline:

### skill_check

| Requirement | Value |
|---|---|
| Command | `sf ai skills check --context=ci --format=junit` |
| Runs when | Changes under `.claude/**` (any skill, rule, or settings file) |
| Timeout | ≤ 10 minutes |
| Failure semantics | `soft_fail: true` — maps to the contract's `enforcement.check.ci` (advisory by default; flipping to blocking is a contract change, not a per-repo decision) |
| Output format | JUnit XML so Buildkite's GitHub-Check annotations surface per-finding context |
| GitHub visibility | `notify: github_check` named `"Skill Validation"` — must surface as its own top-level check in the PR's "Checks" list |

### skill_eval

| Requirement | Value |
|---|---|
| Command | Repo-specific eval driver script (see ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). Calls vanilla `claude -p` with Cortex env vars; same prompts + scoring + thresholds as the CLI. |
| Runs when | Changes under `.claude/skills/**` only (rules and settings don't have evals) |
| Timeout | ≤ 30 minutes |
| Failure semantics | `soft_fail: true` — maps to `enforcement.eval.ci` (advisory by default) |
| Output | Eval summary on stdout (visible in the Buildkite step log). Posting it as a PR comment is recommended but adapter-specific — wire the script's stdout into your repo's existing PR-comment mechanism if you have one. |
| GitHub visibility | `notify: github_check` named `"Skill Eval"` — must surface as its own top-level check in the PR's "Checks" list |

### About GitHub visibility — why it's a requirement

Without `notify: github_check`, a Buildkite step's result folds into
whatever umbrella check the pipeline reports (in snowdev's case,
something like `pull-request-basic`). A developer reviewing a PR
sees only the umbrella; they have to drill in to find whether skill
validation / eval ran or what they reported. That destroys the
feedback loop these checks exist to create.

Each step MUST have its own `notify: github_check` block with a
human-readable `name:`. Those names appear verbatim as top-level
checks in the GitHub PR UI — developers see "Skill Validation" and
"Skill Eval" in the same list as other status checks, with
click-through to the Buildkite build log.

### Prerequisites — what each step needs on the runner

The two steps have different runtime needs:

- **`skill_check`** invokes `sf` directly. The bootstrap MUST
  confirm the runner image has `sf-cli` installed (or a known-good
  install path exists) before proposing the step. If `sf` is
  missing, the step fails at `command not found: sf`; with
  `soft_fail: true` that failure is masked and the step silently
  provides zero signal on every PR. Solution: add an install step
  or update the runner image as part of the bootstrap PR. Grep the
  repo for how it installs tooling today (`apt-get install`,
  `dockerfiles`, `shell.nix`, `asdf`, prebuilt images referenced in
  `agents:` blocks) and propose adding `sf-cli` the same way.
  (The wrapper-script precommit hooks skip gracefully when `sf` is
  missing — they check `command -v sf` — but the Buildkite step
  does not.)
- **`skill_eval`** does NOT invoke `sf`. It runs a Python driver
  script that calls vanilla `claude -p` with Cortex env vars (see
  ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). The runner needs:
  Python (or the runtime the driver script is written in), the
  `claude` CLI, and the secret-fetching path your repo uses to
  resolve `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` at runtime
  (Buildkite Vault plugin, env-var injection from the agent, etc.
  — match what your other steps already do).

Surface both prerequisite sets to the user when proposing — don't
guess at their runner provisioning.

### Why a script, not `sf ai skills eval`?

`sf ai skills eval` works fully on cloud workspaces but is **degraded
on CI workers today** (Buildkite, Jenkins, GitHub Actions — anywhere
that isn't a cloud workspace). The cause is structural, not a bug:
the eval CLI shells out to `sf ai claude -p` to ask Claude for routing
decisions, and `sf ai claude` depends on Cortex routing + `sf`-managed
auth that's only provisioned on cloud workspaces. On a CI worker,
`sf ai claude -p` returns a degraded response with no
`SELECTED_SKILL: ...` line, every prompt parses as `parse_error`, and
the run reports FAIL on routing-correct skills.

**Workaround.** Drive a script that calls vanilla `claude -p` directly
with `ANTHROPIC_BASE_URL=<Cortex endpoint>` and
`ANTHROPIC_AUTH_TOKEN=<Snowhouse session token>` set in the env. Same
prompts, same scoring logic, same contract thresholds — the signal is
equivalent to a working `sf ai skills eval` run.

**Example.** A working driver lives in the `snowflake-eng/snowflake`
monorepo:

- [`.snowci/commands/skill_eval_runner.py`](https://github.com/snowflake-eng/snowflake/blob/main/.snowci/commands/skill_eval_runner.py) — CI entry point.
- [`.snowci/commands/skill_eval_lib.py`](https://github.com/snowflake-eng/snowflake/blob/main/.snowci/commands/skill_eval_lib.py) — Cortex auth + `claude -p` calls + scoring.
- [`.snowci/commands/skill_check.py`](https://github.com/snowflake-eng/snowflake/blob/main/.snowci/commands/skill_check.py) — sibling thin shim to `sf ai skills check`. Useful as a comparison: `skill_check` is LLM-free and shells straight to the CLI; only the eval side needs the Vault/Cortex driver.
- [`.buildkite/steps/skill_eval.yml`](https://github.com/snowflake-eng/snowflake/blob/main/.buildkite/steps/skill_eval.yml) — how the script is wired into a Buildkite step (Buildkite-specific; ignore for GHA/Jenkins adopters).

These are an **example, not a source of truth**. Adopters read them,
then port the scoring + threshold logic to their repo's conventions
(secret fetching, parallelism, output capture, PR-comment posting).
Every adopter's auth and posting paths are different.

**Transition state.** `sf ai skills eval --context=ci` is the eventual
target — the sf team's Jenkins-compatible auth path is in review with
ProdSec. When it ships, this section is replaced by "use the CLI
directly". Until then: scripts.

## Example — snowdev

Snowdev's pipelines live under `.buildkite/pipelines/<pipeline>/`
and use a shared `${GLOBAL_PLUGIN}/changed_target_discovery` plugin
for change-based routing rather than raw `if_changed:` fields.
Snowdev's `skill_check` + `skill_eval` entries land inline in
`.buildkite/pipelines/snowdev-pre-merge/pipeline.yml` alongside
other change-routed steps, using that plugin's matchers.

The shape of a Buildkite step that satisfies the requirements
above (adapt the change-routing syntax to your repo's convention):

```yaml
- label: "Skill Validation"
  key: skill_check
  # CHANGE-ROUTING goes here — use your repo's convention.
  # Examples: native `if_changed:`, plugin-based matchers,
  # unconditional step + in-command skip, monorepo-diff plugin, etc.
  soft_fail: true
  timeout_in_minutes: 10
  agents:
    # Match your repo's existing agent-pool selectors.
    worker_arch: arm
    worker_clone_strategy: full
    worker_size: s
  notify:
    - github_check:
        name: "Skill Validation"
  command: sf ai skills check --context=ci --format=junit

- label: "Skill Eval"
  key: skill_eval
  # Scoped to `.claude/skills/**` (narrower than skill_check)
  soft_fail: true
  timeout_in_minutes: 30
  agents:
    worker_arch: arm
    worker_clone_strategy: full
    worker_size: s
  notify:
    - github_check:
        name: "Skill Eval"
  # Calls the repo's eval driver script, NOT `sf ai skills eval --context=ci`.
  # See "Why a script, not `sf ai skills eval`?" above for the cause and
  # https://github.com/snowflake-eng/snowflake/blob/main/.snowci/commands/skill_eval_runner.py
  # as an example to adapt.
  #
  # The driver script needs ANTHROPIC_BASE_URL and ANTHROPIC_AUTH_TOKEN
  # (Cortex endpoint + Snowhouse session token) in the env. Resolve them
  # via the same secret-fetching path your other Buildkite steps use —
  # e.g. the Vault plugin (`plugins: - vault: ...`), agent-injected env
  # vars, or a wrapper that pulls from your secrets store before exec.
  command: <repo-path-to-eval-driver-script>
```

## Adapting to your repo

1. **Find where this repo's Buildkite pipelines actually live.**
   Common shapes:

   | Layout | Typical path |
   |---|---|
   | Single-file | `.buildkite/pipeline.yml` |
   | Per-pipeline subdirs | `.buildkite/pipelines/<name>/pipeline.yml` |
   | Auto-discovery of steps | `.buildkite/steps/*.yml` |
   | Dynamic pipelines | `.buildkite/pipeline.yml` uploads to `buildkite-agent pipeline upload` at runtime |

2. **Identify the change-routing convention.** What does this
   repo use to run a step only when certain paths change? Options
   in the wild include:
   - Native Buildkite `if_changed:`
   - `${GLOBAL_PLUGIN}/changed_target_discovery` (Snowflake
     monorepo, snowdev)
   - `chronotc/monorepo-diff-buildkite-plugin`
   - Dynamic pipeline-upload with a diff-aware preamble
   - Unconditional steps that skip themselves early in the command

   **Use whichever convention the repo already uses.** Do not
   introduce `if_changed:` into a pipeline that uses
   `changed_target_discovery`; the field will be ignored and the
   step will run on every PR (or never fire, depending on the
   pipeline's runtime).

3. **Match existing agent selectors and worker-pool naming.** The
   `worker_arch`/`worker_size` values in snowdev are Snowflake
   defaults. Your repo may use `queue:` labels or different
   agent-targeting fields.

4. **Realize the requirements above.** The requirements table is
   what must be true of the final step; the example is one valid
   way to structure the YAML. Adapt freely as long as the
   requirements hold.

5. **If the repo has no existing Buildkite CI**, the snowdev
   example is a safe starting point — drop it into a new
   `.buildkite/pipeline.yml` or `.buildkite/steps/` directory.
   Pure-greenfield is the one case where "adopt verbatim" works.

## Idempotency

Same logic applies regardless of layout:

- **Both steps already present and structurally equivalent** →
  skip the addition.
- **Steps present but command/flags diverge** → show diff, ask the
  user to pick or merge.
- **Steps present but the pipeline manifest doesn't reference
  them** → flag for the user; their pipeline discovery may need an
  update before the steps will fire.

## What "soft_fail" actually means here

`soft_fail: true` maps to the contract's advisory enforcement.
When eval scores drop below the PASS threshold, the step exits
non-zero, Buildkite marks the step red, but the overall build
still passes. That's intentional — eval is noisy, and blocking on
eval verdicts would create a false-positive feedback loop that
erodes trust. If the adopter's team wants eval to block, that's a
**contract change**, not a per-repo `soft_fail: false` override.
Raise it in the contract-repo discussion rather than hard-coding
the flip here.
