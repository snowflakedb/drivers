# Precommit hooks — requirements, example, adapt

Two precommit hooks must end up in the adopter's repo:
`sf-ai-skills-check` and `sf-ai-skills-bridge`. Both are
mandatory — the bootstrap does not offer an opt-out. This
reference defines what each hook must do and shows snowdev's
wrapper-script shape as an example. The adopter's repo may have
its own conventions for precommit wrappers, CI-skip guards, and
shell style — match them.

## Requirements (non-negotiable)

### Hook 1 — `sf-ai-skills-check`

| Requirement | Value |
|---|---|
| Command invoked | `sf ai skills check --context=precommit --changed-only --severity=error` |
| Triggers on | Changes under `.claude/**` (the hook's `files:` regex) |
| Failure semantics | Exits non-zero on any error-severity contract finding; fails the commit |
| CI-skip | Must gracefully skip on CI runners without `sf-cli` installed (skip, don't fail) |
| Local-fail | Must hard-fail on developer machines missing `sf-cli` (the gate is real, not optional) |

The three flags carry meaning that must not be dropped:

- **`--context=precommit`** — teaches `--changed-only` to include
  staged-but-uncommitted files (same contract as gofmt /
  golangci-lint: validate the index, not just what's committed).
  Without it, the first commit that introduces a new skill
  slips through because the file isn't in HEAD yet.
- **`--changed-only`** — scopes validation to skills the current
  commit touches. Fast on big monorepos.
- **`--severity=error`** — suppresses warnings and info findings so
  the hook only blocks on real contract violations. Developers can
  still run `sf ai skills check` (no flags) locally for the full
  picture.

### Hook 2 — `sf-ai-skills-bridge`

| Requirement | Value |
|---|---|
| Command invoked | `sf ai skills bridge` (or `sf ai skills bridge --lenient` if the repo keeps real files at `.agents/skills/` deliberately) |
| Triggers on | Changes under `.claude/skills/**` OR `.agents/skills/**` |
| Failure semantics | Exits non-zero if it created, retargeted, or deleted any symlink — developer re-stages regenerated entries (same UX as `gofmt` precommit) |
| CI-skip | Same as above — graceful skip when `sf-cli` is missing in CI |
| Local-fail | Same as above |

The broad `files:` trigger is deliberate: any change under
`.claude/skills/` or `.agents/skills/` must re-run the bridge. A
developer deleting a skill source needs the matching symlink
swept; a developer adding a real file under `.agents/skills/`
needs to be caught by the non-symlink-rejection check.

### Prerequisite — `sf` on the adopter's PATH

Both hooks rely on `sf` being on a local developer's PATH.
Repos that want the convention available to all contributors
should document `sf-cli` install instructions in their onboarding
docs. The hook wrapper handles the "`sf` not installed" case
gracefully on CI, but on developer machines the hook hard-fails
with an install-instructions error message — that's intentional.
A silent-pass-when-missing would let the first contributor
without `sf-cli` ship contract-violating skills without noticing.

## Example — snowdev's wrappers

Snowdev uses a wrapper-script pattern (cloned from the existing
`sf-ai-rules-lint.sh`) with a triple-env-var CI-skip guard
(`CI`, `JENKINS_URL`, `BUILDKITE`). This shape is one valid way
to satisfy the requirements; adapt to your repo's conventions.

### Hook declarations in `.pre-commit-config.yaml`

```yaml
- id: sf-ai-skills-check
  name: Validate Claude skills
  entry: .pre-commit-scripts/sf-ai-skills-check.sh
  language: script
  pass_filenames: false
  files: '^.*\.claude/.*'

- id: sf-ai-skills-bridge
  name: Create .agents/skills/ symlink bridges from .claude/skills/
  entry: .pre-commit-scripts/sf-ai-skills-bridge.sh
  language: script
  pass_filenames: false
  files: '(^|/)(\.claude|\.agents)/skills/'
```

### `.pre-commit-scripts/sf-ai-skills-check.sh`

```bash
#!/usr/bin/env bash
set -e

if ! command -v sf >/dev/null 2>&1; then
    # CI workers typically don't provision sf-cli. Skip there so
    # PRs aren't blocked, but hard-fail locally so the gate is
    # real for contributors.
    if [[ -n "${CI:-}" || -n "${JENKINS_URL:-}" || -n "${BUILDKITE:-}" ]]; then
        echo "sf-cli not on PATH; skipping sf ai skills check in CI." >&2
        exit 0
    fi
    echo "sf-cli is required to validate .claude/skills/ but was not found on PATH." >&2
    echo "Install or build the sf CLI (see dev-env/sf-cli/README.md)." >&2
    exit 1
fi

sf ai skills check --context=precommit --changed-only --severity=error
```

### `.pre-commit-scripts/sf-ai-skills-bridge.sh`

```bash
#!/usr/bin/env bash
set -e

if ! command -v sf >/dev/null 2>&1; then
    if [[ -n "${CI:-}" || -n "${JENKINS_URL:-}" || -n "${BUILDKITE:-}" ]]; then
        echo "sf-cli not on PATH; skipping sf ai skills bridge in CI." >&2
        exit 0
    fi
    echo "sf-cli is required but was not found on PATH." >&2
    echo "Install or build the sf CLI (see dev-env/sf-cli/README.md)." >&2
    exit 1
fi

# Repos that deliberately keep full-copy content at .agents/skills/
# (e.g. generated Codex skills tracked as real files) can set
# SF_AI_SKILLS_BRIDGE_LENIENT=1 to tolerate those entries instead
# of having precommit reject them. Missing bridges are still
# created regardless of mode.
if [[ "${SF_AI_SKILLS_BRIDGE_LENIENT:-}" = "1" ]]; then
    sf ai skills bridge --lenient
else
    sf ai skills bridge
fi
```

Both scripts must be marked executable (`chmod +x`) by the
agent's PR.

## Adapting to your repo

1. **Check existing precommit wrapper conventions.** Look at
   `.pre-commit-scripts/**` in the target repo. Does this repo
   already have wrapper scripts for other CLI tools? Match:
   - Shebang style (`#!/usr/bin/env bash`, `#!/bin/bash`, etc.)
   - Shell-strictness flags (`set -e`, `set -euo pipefail`)
   - Comment style and spacing
   - File naming convention

2. **Match the CI-skip guard pattern.** Snowdev uses three env
   vars (`CI`, `JENKINS_URL`, `BUILDKITE`). Your repo's CI may:
   - Use different env vars (`GITHUB_ACTIONS`, `CIRCLECI`,
     `TEAMCITY_VERSION`, custom internal markers)
   - Already define a canonical "am I in CI?" helper script or
     env var that other hooks check
   - Use a different skip mechanism entirely (e.g. pre-commit's
     `stages: [commit]` and a separate CI config)

   Grep `.pre-commit-scripts/**` and `.github/workflows/**` for
   the pattern this repo already uses. Don't introduce a new
   "how to detect CI" convention — reuse the existing one.

3. **Check `.pre-commit-config.yaml` structure.** Snowdev uses
   multiple `- repo: local` blocks grouped by topic. Other repos
   use:
   - A single large `- repo: local` with all hooks inside
   - `meta` hooks
   - External `- repo:` references pulling hooks from other
     GitHub repos

   Place the new hooks in whichever structure the repo already
   uses. Ask the user if placement is ambiguous.

4. **Install-instructions text.** The local-fail branch tells
   the user where to find `sf-cli` install instructions. Snowdev
   points at `dev-env/sf-cli/README.md` because that's where the
   CLI source lives. In other repos, point at wherever the team's
   onboarding docs direct new contributors to install `sf-cli` —
   which may be an internal wiki page, a release URL, or a
   different README.

5. **If the repo has no existing precommit wrapper pattern**,
   the snowdev example above is a safe starting point — adopt
   verbatim.

## Idempotency

For each hook id, check if it already exists in the target
repo's `.pre-commit-config.yaml`:

- **Both hooks already present** → skip both (no-op).
- **Only one present** → add the missing one; leave the existing
  one alone.
- **Neither present** → add both.

For each wrapper script path, check if the file already exists:

- **Identical content** → skip.
- **Diverged content** → ask user overwrite / keep / merge.
- **Doesn't exist** → create.

## Integration notes

- The hooks go at the end of the `local` repo's hook list in
  `.pre-commit-config.yaml`, not at the top — that groups related
  hooks together, matching snowdev's ordering convention. Your
  repo may have a different convention; honor it.
- Deprecated `sf-ai-rules-build` or `sf-ai-rules-lint` hooks are
  left alone by `bootstrap-skills-evals` — that's
  `migrate-repo-to-skills`'s concern (specifically, the
  deprecated-sweep PR). `sf ai rules lint` in particular must be
  **rescoped, not removed** — it still validates Arctic Owl
  reviewer configs under `.ai/review/`, and silently dropping
  that validation breaks PR review automation.
