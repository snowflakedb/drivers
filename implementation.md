# Mirror Implementation — Follow-ups

Tracks work needed to take the initial Copybara mirror setup (see
`copy.bara.sky` and `.github/workflows/mirror.yml`) from "test destination"
to production-ready.

## Filter model: denylist + manifest snapshot

`copy.bara.sky` uses a **denylist** (`EXCLUDED_PATHS`) rather than a
per-extension allowlist. New source files reach the mirror by default;
anything matching the denylist does not. The denylist stays small and
reviewable.

Internal material should live under `_internal/` — the single catch-all
denylist entry that covers the convention. Existing internal directories
(`adr/`, `doc/`, `.ai/`, `.cursor/`, …) and the mirror machinery itself
are listed explicitly alongside it. Name-based catch-alls (`**/*.internal.*`,
`**/*_internal*`, `**/SECURITY_ASSESSMENT*`, etc.) guard against sensitive
files added inside otherwise-public directories.

A checked-in snapshot `.mirror/manifest.txt` records every file Copybara
would mirror, plus a trailing `# sha256:` line. The
`validations.yml::mirror-manifest` CI check regenerates the snapshot and
fails the PR if it differs from what's committed. Every PR that changes
the mirrored set therefore includes a visible manifest diff the reviewer
must approve — that is the explicit security gate.

**Typical flows:**

- Adding a file inside an already-mirrored directory:
  `scripts/mirror/generate_manifest.py` and commit the updated
  `.mirror/manifest.txt`.
- Adding a new internal-only file: put it under `_internal/`, then
  regenerate the manifest (it will be unchanged — that's the point).
- Adding a new internal-only top-level directory: either create it
  under `_internal/`, or add an explicit entry to `EXCLUDED_PATHS` in
  `copy.bara.sky`, then regenerate.
- CI fails with "manifest is out of date": run
  `scripts/mirror/generate_manifest.py` locally and commit the updated
  `.mirror/manifest.txt`.

## Outstanding items

### Migrate existing internal directories into `_internal/`

`adr/`, `doc/`, `.ai/`, `.cursor/` are denied by explicit top-level
entries. Collapsing them into `_internal/adr/`, `_internal/doc/`, etc.
shrinks `EXCLUDED_PATHS` to essentially just `_internal/**`. Worth doing
once tooling that references those paths (Cursor rules, review configs)
can be updated to follow.

### Validate the inbound (external PR) flow end-to-end

`copy.bara.sky` now has an `import` workflow and
`.github/workflows/mirror-inbound.yml` dispatches it. Before relying on
this in production:

- The inbound workflow uses two tokens: `SNOWFLAKEDB_TOKEN` for the
  origin side (reading the mirror PR from `snowflakedb/ud-mirror-test`
  via git fetch + PR-metadata API) and `SNOWFLAKE_EMU_TOKEN` for the
  destination push into `snowflake-eng/universal-driver`. Confirm
  `SNOWFLAKEDB_TOKEN` has read on the mirror and `SNOWFLAKE_EMU_TOKEN`
  has `contents:write` + PR-create rights on the internal repo.
- Walk one real PR end-to-end: open test PR on mirror → label
  `ok-to-import` → dispatch `mirror-inbound.yml` with the PR number →
  verify the imported PR appears on the internal repo with the external
  contributor's authorship preserved → run internal CI → merge →
  confirm the outbound mirror pushes the merge back and the original
  mirror PR can be closed with a link to the mirrored commit.
- Decide on the label name. Using `ok-to-import` per the design doc;
  if the mirror uses a different convention, update
  `required_labels` in `copy.bara.sky`.
- Consider adding a mirror-side GitHub Actions workflow that comments
  on the imported PR once the internal merge lands, to close the loop
  automatically instead of manually.

### Mirror `.github/` once the bot has the `workflow` scope

`.github/` is currently fully excluded via `EXCLUDED_PATHS` in
`copy.bara.sky`. When the mirror bot attempted to push workflow files
using a classic PAT, GitHub rejected the push:

```
! [remote rejected] HEAD -> main
  (refusing to allow a Personal Access Token to create or update workflow
   `.github/workflows/_build-python-wheels.yml` without `workflow` scope)
```

To restore `.github/` mirroring:

- Switch `SNOWFLAKEDB_TOKEN` (the token that actually pushes to the
  mirror) to either a GitHub App installation token (preferred) or a
  fine-grained PAT that grants the `workflow` scope on
  `snowflakedb/ud-mirror-test`.
- In `copy.bara.sky`: narrow the `.github/**` entry in `EXCLUDED_PATHS`
  to just `.github/workflows/mirror*.yml` (that one stays excluded so
  the mirror's own workflow doesn't get pushed back to itself; the
  existing `verify_match` exclude already covers its
  `snowflake-eng/universal-driver` references).
- Regenerate `.mirror/manifest.txt` — the manifest diff will be large
  the first time and is the reviewable moment for exposing `.github/`
  contents publicly.

### Shipping `close-imported-pr.yml` to the mirror

`close-imported-pr.yml` is the mirror-side half of the inbound loop: it
runs on the mirror after the outbound Copybara push and auto-closes the
original PR with a link to the replayed commit. For the outbound mirror
to *install* it on the mirror repo (i.e. push a file under
`.github/workflows/` to `snowflakedb/ud-mirror-test`), `SNOWFLAKEDB_TOKEN`
must carry the `workflow` scope — the same GitHub restriction as the
`.github/` item above. The current `SNOWFLAKEDB_TOKEN` is a classic PAT
without that scope, so the file is not yet mirrored automatically.

Until the scope is added, probably issue a **separate token** dedicated
to this push rather than broadening the main mirror token — the
outbound mirror runs daily and only needs `contents:write`, while
pushing workflow files is a rarer, higher-privilege operation that
deserves its own auditable credential (e.g. `SNOWFLAKEDB_WORKFLOW_TOKEN`)
scoped to just `snowflakedb/ud-mirror-test` with `workflow`. Short
term: deploy `close-imported-pr.yml` to the mirror manually; long term:
add the extra token (or a GitHub App with the right permission) and
drop the `.github/workflows/close-imported-pr.yml` entry from
`EXCLUDED_PATHS` so it ships via the normal mirror path.



### Branch protection on the mirror destination

The mirror pushes directly to `main` on the destination repo — there is no
PR gate. Without branch protection, anyone with write access (or a
compromised token) can push commits that will then race with or silently
diverge from the mirror bot's pushes.

Configure branch protection on the destination repo (initially
`snowflakedb/ud-mirror-test`, later `snowflakedb/universal-driver`) so
that:

- `main` and `release/*` accept pushes **only** from the mirror bot
  identity (GitHub App installation or deploy-key principal used by
  `SNOWFLAKEDB_TOKEN`).
- Direct pushes from all human users are blocked.
- Force-push is disabled for everyone except the mirror bot (Copybara
  rewrites history on transformation changes, so the bot itself needs
  force-push — scope this narrowly).
- Branch deletion is disabled.
- Tags pushed by the mirror bot are allowed; tag pushes from other
  principals are blocked.

Verify by attempting a direct push from a non-bot account after the rules
are in place — it should be rejected.
