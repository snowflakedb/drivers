# Jenkins CI integration — requirements, example, adapt

This reference tells the bootstrap proposal how to add two stages
(`skill-check` and `skill-eval`) to a Jenkins-using repo. It is
deliberately **not** a paste-this-Jenkinsfile template. Every
Jenkins repo organizes pipelines differently — declarative vs
scripted, monolithic vs library-shared, multibranch vs job DSL —
so dropping snowflake-shaped Groovy into a repo with a different
convention produces stages that look right and don't fire (or
fire on the wrong changes). The bootstrap's job is to realize the
requirements below using whichever convention the adopter's repo
already uses.

Jenkins is a first-class supported flavor alongside Buildkite and
GitHub Actions. Pick this flavor when the repo already has Jenkins
running its CI/CD — adding `skill-check` + `skill-eval` to the
existing Jenkinsfile is far less friction than introducing a
second CI system.

## Requirements (non-negotiable)

Two stages must end up in the adopter's Jenkins pipeline:

### skill-check

| Requirement | Value |
|---|---|
| Command | `sf ai skills check --context=ci --format=junit` |
| Runs when | Changes under `.claude/**` (any skill, rule, or settings file) |
| Timeout | ≤ 10 minutes |
| Failure semantics | Advisory — `catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE')` is the Jenkins idiomatic equivalent of Buildkite's `soft_fail: true`. The stage may go yellow but the build still passes. Maps to the contract's `enforcement.check.ci` (advisory by default; flipping to blocking is a contract change, not a per-repo decision). |
| Output format | JUnit XML — publish via `junit` step so failures surface as test reports in the build summary |

### skill-eval

| Requirement | Value |
|---|---|
| Command | Repo-specific eval driver script (see ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). Calls vanilla `claude -p` with Cortex env vars; same prompts + scoring + thresholds as the CLI. |
| Runs when | Changes under `.claude/skills/**` only (rules and settings don't have evals) |
| Timeout | ≤ 30 minutes |
| Failure semantics | Same advisory `catchError` wrapper as `skill-check`. Maps to `enforcement.eval.ci` (advisory by default) |
| Output | Eval summary on stdout (visible in the Jenkins build console). Posting it as a PR comment is recommended but adapter-specific — wire the script's stdout into your repo's existing PR-comment mechanism (`githubChecksPublisher`, an `httpRequest` to the GitHub Checks API, or whatever your other Jenkins stages use). |

### About GitHub PR visibility

Unlike GitHub Actions (which produces per-job PR checks
automatically) and Buildkite (which uses `notify: github_check`),
Jenkins requires explicit configuration to surface stage results
as GitHub PR checks. Common options in the wild:

- `githubChecksPublisher` step (Jenkins GitHub Checks plugin)
- `httpRequest` to the GitHub Checks API directly
- The repo's existing PR-feedback mechanism, whatever it is

This is an **adapt-to-repo** concern, not a hard requirement. If
the repo already surfaces other Jenkins stages as PR checks, use
the same path for `skill-check` / `skill-eval`. If it doesn't,
the build console output is acceptable — the contract requires
the stages to run and produce signal, not specifically that the
signal lands in a GitHub PR check.

### Prerequisites — what each stage needs on the worker

The two stages have different runtime needs:

- **`skill-check`** invokes `sf` directly. Unlike Buildkite
  runners and GitHub Actions runners (which the bootstrap can
  assume have `sf` provisioned via the runner image), **Jenkins
  workers don't have `sf` pre-installed by default**. The stage
  will fail at `command not found: sf` unless `sf` is provisioned
  via `withSfCli` (see below).
- **`skill-eval`** does NOT invoke `sf`. It runs a driver script
  (typically Python) that calls vanilla `claude -p` with Cortex
  env vars (see ["Why a script, not `sf ai skills eval`?"](#why-a-script-not-sf-ai-skills-eval) below). The
  worker needs Python (or the driver's runtime), the `claude`
  CLI, and a `withCredentials` block (or equivalent) that exports
  `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` from your Jenkins
  credentials store before the script runs.

#### `sf` provisioning for `skill-check`

`snowflake-eng/jenkins_utils` ships a `pipeline-utils` shared
library exposing `DevEnvUtils.withSfCli`, which handles
provisioning end-to-end:

```groovy
@Library('pipeline-utils')
import com.snowflake.DevEnvUtils

new DevEnvUtils().withSfCli {
    sh '''
        set -euo pipefail
        sf artifact oci auth
        sf <your commands>
    '''
}
```

What `withSfCli` does:
- Downloads `sf` from the internal S3 distribution.
- Installs to `/home/jenkins/sf-cli`.
- Generates an artifact cert and sets `SF_CERT_FILE`,
  `SF_KEY_FILE`, `SID_TOKEN_PATH`, and updates `PATH`.
- Runs the closure body.
- Cleans up (uninstalls `sf` and removes the cert) on exit, even
  if the closure throws.

Concrete usage:
[`snowflake-eng/jenkins_utils/jobs/pipelines/SUT/CopyImageToS3.groovy:45`](https://github.com/snowflake-eng/jenkins_utils/blob/main/jobs/pipelines/SUT/CopyImageToS3.groovy#L45).

Use `withSfCli` — don't reinvent provisioning.

### Why a script, not `sf ai skills eval`?

`sf ai skills eval` works fully on cloud workspaces but is **degraded
on CI workers today** (Jenkins, Buildkite, GitHub Actions — anywhere
that isn't a cloud workspace). The cause is structural, not a bug:
the eval CLI shells out to `sf ai claude -p` to ask Claude for routing
decisions, and `sf ai claude` depends on Cortex routing + `sf`-managed
auth that's only provisioned on cloud workspaces. On a Jenkins worker,
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
flavor-agnostic. Adapt the Jenkins invocation: provision Python on the
worker (or use the runtime your existing stages use), fetch Cortex
credentials via your repo's secrets path (Jenkins credentials store
or a `withCredentials` block instead of Buildkite's Vault helper),
then run the script inside `withSfCli { ... }` (or your equivalent
provisioning path). The Buildkite step YAML alongside it doesn't
apply here.

These are an **example, not a source of truth**. Adopters read them,
then port the scoring + threshold logic to their repo's conventions
(secret fetching, parallelism, output capture, PR-comment posting).
Every adopter's auth and posting paths are different.

**Transition state.** `sf ai skills eval --context=ci` is the eventual
target — the sf team's Jenkins-compatible auth path is in review with
ProdSec. When it ships, this section is replaced by "use the CLI
directly". Until then: scripts.

## Example — Jenkinsfile shape

The shape of stages that satisfy the requirements above, using
the `withSfCli` helper. Adapt the agent selector, the change
routing, and the PR-comment posting to your repo's existing
conventions:

```groovy
@Library('pipeline-utils')
import com.snowflake.DevEnvUtils

pipeline {
    agent { label 'your-existing-agent-label' }

    stages {
        stage('skill-check') {
            when {
                anyOf {
                    changeset '.claude/**'
                }
            }
            options { timeout(time: 10, unit: 'MINUTES') }
            steps {
                catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE') {
                    new DevEnvUtils().withSfCli {
                        sh '''
                            set -euo pipefail
                            sf artifact oci auth
                            sf ai skills check --context=ci --format=junit > skill-check.xml
                        '''
                    }
                }
            }
            post {
                always {
                    junit allowEmptyResults: true, testResults: 'skill-check.xml'
                }
            }
        }

        stage('skill-eval') {
            when {
                anyOf {
                    changeset '.claude/skills/**'
                }
            }
            options { timeout(time: 30, unit: 'MINUTES') }
            steps {
                catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE') {
                    new DevEnvUtils().withSfCli {
                        // Calls the repo's eval driver script, NOT
                        // `sf ai skills eval --context=ci`. See "Why a script,
                        // not `sf ai skills eval`?" above for the cause and
                        // https://github.com/snowflake-eng/snowflake/blob/main/.snowci/commands/skill_eval_runner.py
                        // as an example to adapt. The driver script needs
                        // ANTHROPIC_BASE_URL / ANTHROPIC_AUTH_TOKEN exported
                        // from your Jenkins credentials store before it runs.
                        sh '''
                            set -euo pipefail
                            <repo-path-to-eval-driver-script>
                        '''
                    }
                }
            }
        }
    }
}
```

## Adapting to your repo

1. **Find where the Jenkinsfile actually lives.** Common shapes:

   | Layout | Typical path |
   |---|---|
   | Single root Jenkinsfile | `Jenkinsfile` at repo root |
   | Per-pipeline subdirs | `pipelines/<name>/Jenkinsfile` or `jenkins/<name>.groovy` |
   | Multibranch via Job DSL | Pipeline definitions in `jobs/*.groovy`, picked up by a Jenkins job |
   | Shared-library only | Repo has no Jenkinsfile; pipelines live in a separate shared-library repo |

   **In the shared-library case**, the bootstrap must NOT add a
   Jenkinsfile to the adopter repo — surface the finding and ask
   the user to add the stages to the upstream library instead.

2. **Identify the change-routing convention.** Jenkins-native is
   `when { changeset '...' }`, but in the wild you'll see:
   - `when { changeRequest() }` + custom diff scripts
   - `script { if (sh(returnStdout: true, script: 'git diff --name-only ...').contains('.claude/')) { ... } }`
   - Plugin-driven: `pipeline-restful-api`, `path-based-builds-plugin`
   - Multibranch + branch sources doing the filtering before the
     Jenkinsfile even runs

   Use whichever convention the existing pipeline uses. Don't
   introduce `when { changeset }` into a pipeline that uses a
   custom diff helper — the field will be ignored or misfire.

3. **Match existing agent selectors.** Jenkins agents use
   `label`-based selection or `node('label')` blocks. Match what
   the rest of the pipeline does — don't introduce a new label.

4. **Match existing post-build PR-feedback paths.** If the repo
   already publishes other stages as PR checks via
   `githubChecksPublisher` or a custom posting helper, use the
   same path for `skill-check` / `skill-eval`. Don't add a new
   posting mechanism just for these two stages.

5. **Realize the requirements above.** The requirements table is
   what must be true of the final stages; the example is one
   valid way to structure the Groovy. Adapt freely as long as
   the requirements hold.

6. **If the repo has a Jenkinsfile but no GitHub PR-checks
   integration today**, the bootstrap should still add the stages
   — the build-console output is acceptable signal. Note this
   in the proposal so the user can plan a follow-up to wire up
   PR checks if they want.

## Idempotency

Same logic as the other flavors:

- **Both stages already present and structurally equivalent** →
  skip the addition.
- **Stages present but command/flags diverge** → show diff, ask
  the user to pick or merge.
- **Stages defined but not invoked from the active pipeline
  block** → flag for the user; the pipeline's stage list may need
  an update before the new stages will fire.

## What "advisory" means in Jenkins

`catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE')`
maps to the contract's advisory enforcement. When eval scores
drop below the PASS threshold, the stage exits non-zero, Jenkins
marks the stage UNSTABLE (yellow ball), but the overall build
result stays SUCCESS — no PR block. That's intentional: eval is
noisy, and blocking on eval verdicts would create a false-positive
feedback loop that erodes trust. If the adopter's team wants eval
to block, that's a **contract change**, not a per-repo override.
Raise it in the contract-repo discussion rather than swapping
`catchError` for a hard `sh ... || exit 1` here.
