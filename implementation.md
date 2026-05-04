# Mirror Implementation — Follow-ups

Tracks work needed to take the initial Copybara mirror setup (see
`copy.bara.sky` and `.github/workflows/mirror.yml`) from "test destination"
to production-ready.

## Outstanding items

### Validate the inbound (external PR) flow end-to-end

`copy.bara.sky` now has an `import` workflow and
`.github/workflows/mirror-inbound.yml` dispatches it. Before relying on
this in production:

- Confirm the `SNOWFLAKE_EMU_TOKEN` used by the workflow has read on the
  mirror repo *and* `contents:write` + PR create rights on the internal
  repo. The outbound mirror only needs one of those; inbound needs both.
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

`.github/` (workflows, composite actions, labeler, copilot-instructions,
etc.) is currently excluded from the Copybara allowlist. When the mirror
bot attempted to push workflow files using a classic PAT, GitHub rejected
the push:

```
! [remote rejected] HEAD -> main
  (refusing to allow a Personal Access Token to create or update workflow
   `.github/workflows/_build-python-wheels.yml` without `workflow` scope)
```

To restore `.github/` mirroring:

- Switch `MIRROR_GITHUB_TOKEN` / `SNOWFLAKE_EMU_TOKEN` to either a
  GitHub App installation token (preferred) or a fine-grained PAT that
  grants the `workflow` scope on the destination repo.
- Re-add the `.github/**/*.yml`, `.github/**/*.yaml`, `.github/**/*.sh`,
  `.github/**/Dockerfile`, `.github/secrets/*.gpg`, and
  `.github/copilot-instructions.md` entries to `origin_files` in
  `copy.bara.sky`.
- Decide whether the mirror workflow itself (`.github/workflows/mirror.yml`)
  should be mirrored. If yes, keep the existing `verify_match` exclude
  for `.github/workflows/mirror*` so its references to
  `snowflake-eng/universal-driver` don't trip the guard.



### Branch protection on the mirror destination

The mirror pushes directly to `main` on the destination repo — there is no
PR gate. Without branch protection, anyone with write access (or a
compromised token) can push commits that will then race with or silently
diverge from the mirror bot's pushes.

Configure branch protection on the destination repo (initially
`snowflake-eng/universal-driver-mirror-test`, later
`snowflakedb/universal-driver`) so that:

- `main` and `release/*` accept pushes **only** from the mirror bot
  identity (GitHub App installation or deploy-key principal used by
  `MIRROR_GITHUB_TOKEN`).
- Direct pushes from all human users are blocked.
- Force-push is disabled for everyone except the mirror bot (Copybara
  rewrites history on transformation changes, so the bot itself needs
  force-push — scope this narrowly).
- Branch deletion is disabled.
- Tags pushed by the mirror bot are allowed; tag pushes from other
  principals are blocked.

Verify by attempting a direct push from a non-bot account after the rules
are in place — it should be rejected.
