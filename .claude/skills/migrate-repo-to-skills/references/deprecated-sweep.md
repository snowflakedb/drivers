# Deprecated-tooling sweep — first PR of the migration stack

The first PR of the migration stack removes references to the legacy
`sf ai rules build` pipeline and rescopes `sf ai rules lint` to
`.ai/review/` (Arctic Owl configs) only. This PR MUST land before any
batch PR — otherwise every batch will regenerate stale pointer files
under the old layout and blow up precommit / CI on every commit.

## Why this PR is first

`sf ai rules build` walked `.ai/commands/*.md` and generated pointer
files under `.claude/` and `.cursor/`. Under the new layout we
create `.agents/skills/` symlinks with `sf ai skills bridge` instead
— `.cursor/` is no longer written to. Leaving `rules build` wired
into precommit would fight the migration on every commit.

**The replacement is `sf ai skills bridge`**, wired as a new
precommit hook alongside the `sf-ai-skills-check` hook. The
`bridge` hook populates `.agents/skills/` symlinks that point at
each source skill under `.claude/skills/` — one bridge surface
serves Cursor and Codex CLI discovery. See "What to add" below for
the hook YAML. If the adopter's repo was bootstrapped first (via
`sf ai skills repo-setup`), the bridge hook is already in place
and this PR just removes the `rules build` invocation. If
bootstrap hasn't run yet, this PR must add the bridge hook
simultaneously — snowdev and existing adopters should never spend
a commit window with neither `rules build` nor `skills bridge`
wired, because `.agents/skills/` drifts silently in between.

`sf ai rules lint` has two jobs: (1) lint the old pipeline's outputs,
and (2) validate `.ai/review/*.yaml` (Arctic Owl reviewer configs).
Only (2) is still needed post-migration — but (2) is **critical**
and must not be dropped. Arctic Owl configs are the repo's code-
review automation; silently skipping their validation lets broken
configs reach production and breaks PR review without warning.

**The correct action is rescope, not remove.** Pass
`--review-rules-dir .ai/review` and narrow the precommit `files:`
pattern to Arctic Owl configs only. The hook stays wired; only its
scope changes.

## What to remove

Grep for every invocation of `sf ai rules build`. Common targets:

- `.pre-commit-config.yaml` — look for hook `id: sf-ai-rules-build`
  or `entry: sf ai rules build`.
- `.pre-commit-scripts/**` — wrapper scripts that call
  `sf ai rules build`.
- `.github/workflows/**` — any GitHub Actions job that builds rules.
- `.buildkite/**` — Buildkite pipelines.
- `**/*.py` — CI/cron scripts (e.g., knowledge-refresh bots that
  invoke `sf ai rules build` as part of their flow).
- `**/docs/**` — documentation referring to the old command.

Remove the invocation entirely. If the hook/script was only doing
`sf ai rules build`, delete the hook/script. If it did other things
too, strip only the `rules build` step.

## What to rescope

Grep for every invocation of `sf ai rules lint`. Add the flag:

```
sf ai rules lint --review-rules-dir .ai/review
```

Also narrow any precommit `files:` pattern to match only Arctic Owl
config files:

```yaml
files: '^\\.ai/review/.*\\.ya?ml$'
```

Monorepo precedent (working entry to mirror):

```yaml
- id: sf-ai-rules-lint
  name: Validate ArcticOwl AI configs
  entry: sf ai rules lint --review-rules-dir .ai/review
  language: system
  pass_filenames: false
  files: '^\\.ai/review/.*\\.ya?ml$'
```

## What to add

Replace the removed `sf ai rules build` hook with a new precommit
entry for `sf ai skills bridge`:

```yaml
- id: sf-ai-skills-bridge
  name: Create .agents/skills/ symlink bridges from .claude/skills/
  entry: .pre-commit-scripts/sf-ai-skills-bridge.sh
  language: script
  pass_filenames: false
  files: '(^|/)(\\.claude|\\.agents)/skills/'
```

`bridge` exits non-zero if it created, retargeted, or deleted any
symlink, so precommit will fail the commit and tell the user to
re-stage the regenerated `.agents/skills/` entries — same UX as
`gofmt` precommit hooks.

The hook calls a wrapper script under `.pre-commit-scripts/` that
skips gracefully on CI runners without sf-cli installed. See
`.claude/skills/bootstrap-skills-evals/references/precommit-hooks.md`
for the canonical wrapper template.

## Repo-specific nuances

Some repos run `sf ai rules lint` via a wrapper script that skips the
hook on CI runners without sf-cli installed (snowdev has exactly this
shape in `.pre-commit-scripts/sf-ai-rules-lint.sh`). Preserve the
CI-skip guard when rescoping — don't inline the new hook in a way
that removes the `if [[ -n "$CI" ]]; then skip; fi` safety check.

## Verification after this PR lands

```bash
# No hits anywhere in the repo:
rg 'sf ai rules build'
# (should print nothing)

# Precommit works:
pre-commit run --all-files

# Rules lint still validates Arctic Owl configs:
sf ai rules lint --review-rules-dir .ai/review

# Refresh-pointers hook runs:
sf ai skills bridge
# (exits 0 on a clean repo)
```

## PR body checklist for this sweep PR

- [ ] All `sf ai rules build` invocations removed (grep clean).
- [ ] All `sf ai rules lint` invocations rescoped with
      `--review-rules-dir .ai/review`.
- [ ] New `sf-ai-skills-bridge` precommit hook added.
- [ ] CI wrapper-script safety guards preserved.
- [ ] No skill migration in this PR (that's the batch PRs).
- [ ] `pre-commit run --all-files` passes.
