# GitHub Actions CI integration — requirements, example, adapt

This reference tells the bootstrap proposal how to add two CI
workflows (`skill-check` and `skill-eval`) to a GitHub-Actions-using
repo. It is deliberately **not** a paste-this-YAML template. Repos
organize workflows, runner pools, and tooling provisioning
differently; dropping snowdev-shaped YAML into a repo that uses a
different convention produces workflows that run on the wrong
runner, miss dependencies, or fire on the wrong paths. The
bootstrap's job is to realize the requirements below using whichever
convention the adopter's repo already uses.

## Requirements (non-negotiable)

Two workflows must end up in the adopter's repo:

### skill-check

| Requirement | Value |
|---|---|
| Command | `sf ai skills check --context=ci --format=junit` |
| Runs when | PR changes under `.claude/**` (skills, rules, settings) |
| Timeout | ≤ 10 minutes |
| Failure semantics | Advisory via `continue-on-error: true` — matches `enforcement.check.ci: advisory` (a contract change is needed to flip to blocking) |
| Output | JUnit XML (surface per-finding context in PR annotations) |

### skill-eval

| Requirement | Value |
|---|---|
| Command | Repo-specific eval driver script (see ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). Calls vanilla `claude -p` with Cortex env vars; same prompts + scoring + thresholds as the CLI. |
| Runs when | PR changes under `.claude/skills/**` only |
| Timeout | ≤ 30 minutes |
| Failure semantics | Advisory via `continue-on-error: true` — matches `enforcement.eval.ci: advisory` |
| Output | Eval summary on stdout (visible in the workflow run log). Posting it as a PR comment is recommended but adapter-specific — wire the script's stdout into your repo's existing PR-comment mechanism (`actions/github-script`, `peter-evans/create-or-update-comment`, etc.). |

### About GitHub visibility

GitHub Actions surfaces each job as its own top-level check in the
PR's "Checks" list by default — the check's name comes from the
workflow's `name:` field (or the job key if `name:` is absent).
No extra configuration needed.

**Naming requirement:** use human-readable workflow names —
`name: Skill Validation` and `name: Skill Eval` (as shown in the
example below) — so developers see those exact labels in the
checks list. Generic names like `name: CI` or keying jobs by
implementation detail (`name: run-sf-check`) bury the signal.

Contrast with Buildkite: Buildkite steps fold their results into
the pipeline's umbrella status check by default, so Buildkite
requires an explicit `notify: github_check` block per step to
surface the check at the PR level. GH Actions is the simpler case.

### Prerequisites — what each workflow needs on the runner

The two workflows have different runtime needs:

- **`skill-check`** invokes `sf` directly. The runner needs
  `sf-cli` installed.
- **`skill-eval`** does NOT invoke `sf`. It runs a driver script
  (typically Python) that calls vanilla `claude -p` with Cortex
  env vars (see ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). The
  runner needs Python (or the driver's runtime), the `claude` CLI,
  and a step that resolves `ANTHROPIC_BASE_URL` /
  `ANTHROPIC_AUTH_TOKEN` from `secrets.*` into the env before the
  script runs.

The rest of this section is about installing `sf` for the
`skill-check` workflow. **Every repo installs CLI tools in GitHub
Actions differently**, and there is no universal install step that
works out of the box. Options the agent should consider (by asking
the user, not guessing):

- Downloading a prebuilt binary from a GitHub release / internal
  artifact store
- `npm install -g @snowflake-eng/sf-cli` (if the CLI ships there)
- Building from source (bazel, go build)
- Using a preconfigured runner image that already has `sf`
- A custom reusable workflow or composite action the repo already
  uses for CLI tooling

Grep the repo's existing workflows for precedent — how does this
repo install *other* CLI tools today? Match that pattern. If no
precedent exists, ask the user.

**Do NOT commit a placeholder `exit 1` install step.** A broken
install fails the job visibly; a placeholder install that "looks
right" but silently no-ops is worse. If the install step can't be
resolved in the bootstrap PR, flag it as a prerequisite the user
must answer before the workflow lands.

### Why a script, not `sf ai skills eval`?

`sf ai skills eval` works fully on cloud workspaces but is **degraded
on CI workers today** (GitHub Actions, Buildkite, Jenkins — anywhere
that isn't a cloud workspace). The cause is structural, not a bug:
the eval CLI shells out to `sf ai claude -p` to ask Claude for routing
decisions, and `sf ai claude` depends on Cortex routing + `sf`-managed
auth that's only provisioned on cloud workspaces. On a GHA runner,
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

The example lives in a Buildkite-using repo, but the Python files are
flavor-agnostic. Adapt the GHA invocation: install Python on the
runner, fetch Cortex credentials via your repo's secrets path
(`secrets.SOMETHING` instead of Buildkite's Vault helper), then run
the script. The Buildkite step YAML alongside it doesn't apply here.

These are an **example, not a source of truth**. Adopters read them,
then port the scoring + threshold logic to their repo's conventions
(secret fetching, parallelism, output capture, PR-comment posting).
Every adopter's auth and posting paths are different.

**Transition state.** `sf ai skills eval --context=ci` is the eventual
target — the sf team's CI-compatible auth path is in review with
ProdSec. When it ships, this section is replaced by "use the CLI
directly". Until then: scripts.

## Example — reference shape

This example shows the workflow structure that satisfies the
requirements above. Adapt runner selectors, install step, and
permissions to match your repo.

```yaml
name: Skill Validation
on:
  pull_request:
    paths:
      - '.claude/**'
      - '.github/workflows/skill-check.yml'

jobs:
  skill-check:
    # Match your repo's runner convention. Options:
    #   ubuntu-latest (hosted)
    #   self-hosted label(s)
    #   organization-wide runner group
    runs-on: ubuntu-latest
    timeout-minutes: 10
    # Advisory per the contract's enforcement.check.ci default.
    # A contract change is required to flip this to blocking.
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - name: Install sf CLI
        # Replace with your repo's install method — see above.
        run: <your-install-command-here>
      - name: Run sf ai skills check
        run: sf ai skills check --context=ci --format=junit
```

For `skill-eval.yml`, the shape is similar but invokes the driver
script (NOT `sf ai skills eval --context=ci` — see ["Why a script,
not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval)
above) and exports Cortex credentials into the env first:

```yaml
name: Skill Eval
on:
  pull_request:
    paths:
      - '.claude/skills/**'
      - '.github/workflows/skill-eval.yml'

jobs:
  skill-eval:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    # Advisory per the contract's enforcement.eval.ci default.
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - name: Install claude CLI
        # Replace with your repo's install method for the vanilla
        # Anthropic `claude` binary.
        run: <your-claude-install-command-here>
      - name: Run skill-eval driver
        env:
          # Resolve from your repo's secrets store.
          ANTHROPIC_BASE_URL: ${{ secrets.CORTEX_BASE_URL }}
          ANTHROPIC_AUTH_TOKEN: ${{ secrets.CORTEX_AUTH_TOKEN }}
        run: <repo-path-to-eval-driver-script>
```

Adapt: the secret names (`CORTEX_BASE_URL`, `CORTEX_AUTH_TOKEN`)
are placeholders — match what your repo already uses for Cortex
credentials. If your repo posts PR comments via a follow-up step
(e.g. `peter-evans/create-or-update-comment`), add it after the
driver step using its stdout.

## Adapting to your repo

1. **Find where this repo's workflows actually live.** Most repos
   use `.github/workflows/` but some have reusable workflows under
   `.github/workflows/` with callers under a different path.

2. **Check existing workflow conventions.** Look at
   `.github/workflows/*.yml` to learn:
   - What runners does this repo use? `ubuntu-latest`, self-hosted
     pools with specific labels, organization-managed runner groups?
   - What permissions does this repo grant to workflows by default?
     Does it use the modern `permissions:` block, or rely on the
     repo-level default?
   - Does the repo use reusable workflows or composite actions for
     common tasks (checkout + tooling setup)? Use them.
   - How are secrets and GitHub tokens scoped? `skill-eval` needs
     to read Cortex credentials from `secrets.*` for the driver
     script, and — IF the adopter chooses to post stdout as a PR
     comment — the workflow needs `pull-requests: write` (see
     "Permissions note" below).

3. **Install step precedent.** Grep existing workflows for how they
   install CLI tools today. Common patterns:
   - `curl | sh` from a release URL
   - Pre-installed in the runner image (self-hosted pools often)
   - A shared composite action under `.github/actions/`
   - Package-manager installation (`apt-get`, `brew`, `npm`)

   Match the dominant pattern. A one-off install that doesn't fit
   the repo's conventions will surprise reviewers.

4. **Path-filter precision.** `skill-check` runs on all of
   `.claude/**` because rules, settings, and skill content can all
   affect validation. `skill-eval` runs on `.claude/skills/**`
   only — rules and settings don't have routing evals. Don't
   broaden the eval filter; it wastes Claude API budget.

5. **If the repo has no existing GitHub Actions**, the example
   above is a safe starting point once the install step is filled
   in. Pure-greenfield is where "adopt verbatim" works.

## Idempotency

- **Both workflows already present with equivalent commands and
  triggers** → skip the addition.
- **Workflows present but command/flags/paths diverge** → show
  diff, ask the user to pick or merge.
- **Only one workflow present** → add the missing one; leave the
  existing alone.

## Permissions note

If your `skill-eval` workflow posts the driver script's stdout as
a PR comment (via `actions/github-script`,
`peter-evans/create-or-update-comment`, or similar), the workflow
needs `pull-requests: write` permission (or `issues: write`,
depending on how the repo scopes it). If the repo uses the
restrictive default GitHub token, the comment post will silently
fail. Surface this as a required permission in the proposal when
posting is part of the adopter's plan; the bare driver run (just
stdout in the workflow log) doesn't need it.
