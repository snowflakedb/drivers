# Mirroring between snowflake-eng and snowflakedb

This repo (`snowflake-eng/universal-driver`) is the internal source of
truth. A Copybara-based mirror keeps `snowflakedb/universal-driver` in
sync with the public-safe subset of this tree, and lets external
contributors land changes through a labeled-PR import flow.


## How it works

Two directions, both driven by `ci/mirroring/copy.bara.sky`:

- **Outbound** (`mirror` workflow): every commit on `main` or
  `release/*` is replayed onto the same branch on the mirror.
  Authorship and commit message are preserved; Copybara appends a
  `GitOrigin-RevId: <sha>` trailer for traceability. Runs daily via
  `.github/workflows/mirror.yml`, which also accepts a manual
  `workflow_dispatch` (with a `dry_run` input that previews the push
  without writing to the mirror).

- **Inbound** (`import` workflow): an external contributor opens a PR
  on the mirror. A maintainer applies the `ok-to-import` label, then
  dispatches `.github/workflows/mirror-inbound.yml` with the PR
  number. Copybara reads the labeled PR and creates a matching PR on
  the internal repo with the contributor's authorship preserved, so
  it runs against full internal CI before merge. Once the internal PR
  is merged, the next outbound run pushes the merge back to the
  mirror, closing the loop.

### Filter model: denylist + ArcticOwl reviewer

`ci/mirroring/copy.bara.sky` uses a **denylist** (`EXCLUDED_PATHS`) — new source
files reach the mirror by default; anything matching the denylist does
not. The denylist is intentionally short and reviewable; see the
`EXCLUDED_PATHS` value in `ci/mirroring/copy.bara.sky` for the authoritative list.

A denylist defaults to "public" for new files, which is the opposite of
what a mirror config typically wants. Two mechanisms compensate:

1. **`_internal/` convention.** Put any new internal-only material
   under `_internal/` and it is covered by the catch-all entry without
   touching `EXCLUDED_PATHS`. This is the default home for new internal
   content; new top-level exclusions should be rare.

2. **ArcticOwl reviewer.** The `universal-driver-mirror-privacy` rule
   (`.ai/review/universal-driver-mirror-privacy.yaml`, status: enabled)
   reviews every new file in a public directory and flags any whose
   content looks Snowflake-internal (internal repo refs, internal
   hostnames, internal process docs, internal-style names like
   `*_internal*` / `*_security_assessment*`, AI configs that reference
   internal infrastructure). The rule comments on the PR — it does not
   block merging on its own, but a flag is the reviewer's prompt to
   move the file under `_internal/`.

The Copybara workflow itself also runs two `verify_match` checks at
mirror time as a defense-in-depth backstop: any mirrored file that
still references `snowflake-eng/universal-driver` or an internal
hostname (`*.snowflakecomputing.internal`, `*.corp.snowflake`) fails
the sync.

`EXCLUDED_PATH_OVERRIDES` re-includes specific files inside an
otherwise-excluded directory (used today for selected `.cursor/` rules
and commands). Adding a path to that list publishes the file on the
next mirror run.

### Tokens

Two GitHub credentials are used so a compromise of one cannot reach
the other org:

- `DRIVER_MIRROR_TOKEN` — `snowflake-eng` access. Outbound origin
  fetch; inbound destination push.
- `DRIVER_MIRROR_TOKEN_SNOWFLAKEDB` — `snowflakedb` access. Outbound
  destination push; inbound origin fetch + PR-metadata API.

The outbound workflow injects `DRIVER_MIRROR_TOKEN` via a URL-scoped
`http.<URL>.extraHeader` so the credential is bound to the
`snowflake-eng` fetch URL only and never leaks onto the
`snowflakedb` destination push.

## How to use it

### Adding internal-only files

Put new internal files under `_internal/`. Nothing else is required —
the denylist already covers `_internal/**`.

If the file genuinely cannot live under `_internal/` (a new top-level
directory required by external tooling, for example), add an entry to
`EXCLUDED_PATHS` in `ci/mirroring/copy.bara.sky` and document the reason inline.

### Adding files that should be mirrored

Add them as normal. The denylist defaults to public, so the file ships
on the next mirror run. The ArcticOwl reviewer will comment on the PR
if anything in the new file looks internal; respond by either moving
the file or refining the content.

### Adding a mirrored file inside an otherwise-excluded directory

Append the path to `EXCLUDED_PATH_OVERRIDES` in `ci/mirroring/copy.bara.sky`. The
file will be included in the next mirror run.

### Responding to an ArcticOwl mirror-privacy comment

The rule flags content that looks Snowflake-internal in a public path.
Options:

- The flag is correct → move the file under `_internal/` (or rewrite
  the offending content).
- The flag is a false positive → reply on the PR comment explaining
  why; the rule is advisory, not blocking.

### Importing an external PR (inbound)

1. External contributor opens a PR against `main` on the mirror.
2. A maintainer applies the `ok-to-import` label.
3. Dispatch `.github/workflows/mirror-inbound.yml` with the mirror PR
   number.
4. A matching PR appears on the internal repo with the contributor's
   authorship preserved. Internal CI runs against it.
5. On merge, the next outbound run replays the commit back to the
   mirror. Close the original mirror PR with a link to the mirrored
   commit.

## Files involved

- `ci/mirroring/copy.bara.sky` — outbound `mirror` and inbound `import` workflows,
  denylist, overrides.
- `ci/mirroring/Dockerfile.copybara` — pinned Copybara runtime image used by CI
  agents and developer laptops.
- `scripts/mirror/run_copybara.sh` — shared runner script.
- `.ai/review/universal-driver-mirror-privacy.yaml` — ArcticOwl rule
  that reviews new files in public paths for internal-looking content.
- `.github/workflows/mirror.yml` — scheduled outbound run plus
  `workflow_dispatch` with a `dry_run` input.
- `.github/workflows/mirror-inbound.yml` — inbound PR-import
  dispatcher.
- `.github/workflows/mirror-tokens-check.yml` — token health probe.
- `.github/workflows/validations.yml::mirror-manifest` — drift gate.

## Remaining steps

Tracked here so the rollout is visible. None block routine use of the
mirror.

- **Mirror `.github/` once the bot has the `workflow` scope.** Today
  `.github/**` is fully excluded because the classic-PAT mirror token
  cannot push workflow files (GitHub rejects without the `workflow`
  scope). Switch `DRIVER_MIRROR_TOKEN_SNOWFLAKEDB` to a GitHub App
  installation token (preferred) or a fine-grained PAT with the
  `workflow` scope, then narrow the `.github/**` entry in
  `EXCLUDED_PATHS` to just `.github/workflows/mirror*.yml` and
  regenerate the manifest. The first manifest diff will be large and
  is the reviewable moment for exposing `.github/` publicly.
- **Validate the inbound flow end-to-end.** Walk one real PR
  (open on mirror → label → dispatch inbound → run internal CI →
  merge → confirm outbound replays it back → close the mirror PR).
  Confirm `DRIVER_MIRROR_TOKEN_SNOWFLAKEDB` has read on the mirror
  and `DRIVER_MIRROR_TOKEN` has `contents:write` + PR-create rights
  on the internal repo. Decide on the final label name (currently
  `ok-to-import` per the design doc) and update `required_labels`
  in `copy.bara.sky` if it changes.
- **Ship `close-imported-pr.yml` to the mirror.** It is the
  mirror-side half of the inbound loop: runs on the mirror after the
  outbound push and auto-closes the original PR with a link to the
  replayed commit. Pushing it requires the same `workflow` scope as
  above; consider issuing a separate dedicated token
  (`DRIVER_MIRROR_WORKFLOW_TOKEN`) for the higher-privilege workflow
  pushes rather than broadening the daily mirror token. Until then,
  deploy the file to the mirror manually.
- **Move pipelines to Buildkite.** A separate PR adds
  `.buildkite/pipelines/mirror-{outbound,inbound}/` definitions; once
  validated they will be the primary execution path with the GitHub
  Actions copies kept as a `workflow_dispatch` escape hatch.
