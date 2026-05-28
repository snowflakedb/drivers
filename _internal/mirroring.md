# Mirroring between snowflake-eng and snowflakedb

This repo (`snowflake-eng/universal-driver`) is the internal source of
truth. A Copybara-based outbound mirror keeps
`snowflakedb/universal-driver` in sync with the public-safe subset of
this tree.


## How it works

`ci/mirroring/copy.bara.sky` defines a `mirror` workflow that replays every commit
on `main` and `release/*` onto the same branch on the mirror.
Authorship and commit message are preserved; Copybara appends a
`GitOrigin-RevId: <sha>` trailer for traceability. Runs daily via
`.github/workflows/mirror.yml`, which also accepts a manual
`workflow_dispatch` (with a `dry_run` input that previews the push
without writing to the mirror).

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

- `DRIVER_MIRROR_TOKEN` — `snowflake-eng` access. Used as the
  outbound origin fetch credential.
- `DRIVER_MIRROR_TOKEN_SNOWFLAKEDB` — `snowflakedb` access. Used as
  the outbound destination push credential.

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

## Files involved

- `ci/mirroring/copy.bara.sky` — outbound `mirror` workflow, denylist, overrides.
- `ci/mirroring/Dockerfile.copybara` — pinned Copybara runtime image used by CI
  agents and developer laptops.
- `scripts/mirror/run_copybara.sh` — shared runner script.
- `.ai/review/universal-driver-mirror-privacy.yaml` — ArcticOwl rule
  that reviews new files in public paths for internal-looking content.
- `.github/workflows/mirror.yml` — scheduled outbound run plus
  `workflow_dispatch` with a `dry_run` input.

## Remaining steps

Tracked here so the rollout is visible. None block routine use of the
outbound mirror.

- **Mirror `.github/` once the bot has the `workflow` scope.** Today
  `.github/**` is fully excluded because the classic-PAT mirror token
  cannot push workflow files (GitHub rejects without the `workflow`
  scope). Switch `DRIVER_MIRROR_TOKEN_SNOWFLAKEDB` to a GitHub App
  installation token (preferred) or a fine-grained PAT with the
  `workflow` scope, then narrow the `.github/**` entry in
  `EXCLUDED_PATHS` to just `.github/workflows/mirror*.yml`.
- **Add the inbound (external PR import) flow.** A separate PR adds an
  `import` workflow to `ci/mirroring/copy.bara.sky` and a `mirror-inbound.yml`
  dispatcher so external contributors can land changes through a
  labeled-PR import flow.
- **Move pipelines to Buildkite.** A separate PR adds
  `.buildkite/pipelines/mirror-{outbound,inbound}/` definitions; once
  validated they will be the primary execution path with the GitHub
  Actions copies kept as a `workflow_dispatch` escape hatch.
