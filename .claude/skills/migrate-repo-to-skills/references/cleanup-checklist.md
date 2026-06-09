# Cleanup checklist — final PR of the migration stack

Consumed by the downstream agent (Phase 5 handoff) as the spec for the
final PR in the migration stack. Assumes every batch PR has already
landed and that the deprecated-tooling sweep was the first PR. Those
items are NOT repeated here.

## Scope

This PR contains NO new skills. It only:

- Deletes any stragglers under `.ai/commands/` and `.ai/context/`.
- Fixes any remaining orphan references to `.ai/` paths.
- Reconciles `.agents/skills/` bridges via `sf ai skills bridge`
  (serves both Cursor and Codex CLI discovery).
- Verifies `sf ai skills check` passes.

## Ordered checklist

### 1. `.ai/commands/` is empty or deleted

```bash
find .ai/commands -type f 2>/dev/null | head
# (should print nothing)
```

If anything remains, it was missed by a batch PR. Figure out which
batch it belonged to and fold it in retroactively rather than
deleting blind.

### 2. `.ai/context/` is empty or deleted

```bash
find .ai/context -type f 2>/dev/null | head
# (should print nothing)
```

Same caveat — don't delete blindly.

### 3. Zero repo-wide references to `.ai/commands/` or `.ai/context/`

```bash
rg '\.ai/(commands|context)/' \
   --glob '!.ai/review/**' \
   --glob '!.claude/**' \
   --glob '!.cursor/**' \
   --glob '!.doc-plans/**'
# (should print nothing)
```

Targets to scan: every `CLAUDE.md` (ancestor + descendant),
`.claude/agents/**/*.md`, `.claude/commands/**/*.md`,
`.claude/settings*.json`, and any other `*.md` in the repo.

Any hit is a dangling pointer — update it to the new
`.claude/skills/<path>/SKILL.md` or remove it if obsolete.

### 4. Known ancestor `CLAUDE.md` files updated

For snowdev specifically (adjust per-repo):

- `dev-env/CLAUDE.md` — previously referenced
  `.ai/commands/dev-env/`. Update to point to `.claude/skills/dev-env/`.
- `snowci/CLAUDE.md` — previously referenced several
  `.ai/commands/snowci/master-*/` paths. Update all to the new
  `.claude/skills/snowci/` equivalents.

Per-repo: the audit in Phase 1 recorded every such reference. Use
that list.

### 5. `.agents/skills/` is in sync via `sf ai skills bridge`

```bash
sf ai skills bridge
# (must exit 0 on a well-migrated repo)
```

`sf ai skills bridge` populates `.agents/skills/` with symlinks
pointing at each source skill under `.claude/skills/`. One bridge
surface serves both Cursor and Codex CLI discovery (both walk
`.agents/skills/` recursively and follow symlinks).

If this exits non-zero, stage the resulting diff (new symlinks /
deleted orphans) into this cleanup PR. The hook should already
have been added in the first PR of the stack (replacing the old
`sf ai rules build` pointer-generator), but run the command
manually here to be sure.

### 6. `sf ai skills check` passes

```bash
sf ai skills check <repo-root>
```

Compare the finding set against the pre-migration baseline captured
in Phase 1. Acceptable outcomes:

- **Strictly fewer findings** than baseline — great, migration
  improved things.
- **Same findings** — acceptable, migration was neutral.
- **More findings** — investigate each new one before shipping. Most
  likely a frontmatter field was lost in translation.

The attached PR body should include the baseline-vs-now diff.

### 7. Untouched-dirs check

```bash
# These must NOT have been changed by the migration:
git diff <base-branch> HEAD -- \
  .ai/review/ .ai/casper-tasks/ .ai/mcp/ .ai/plans/ \
  .ai/README.md .ai/OWNERS.yml
# (should print nothing)
```

If any of these changed, revert them — they're out of scope for the
migration.

### 8. Deprecated-tooling regression check

```bash
rg 'sf ai rules build'
# (should still print nothing — confirms no batch re-introduced it)

rg 'sf ai rules lint' | rg -v 'review-rules-dir'
# (should print nothing — every lint invocation must carry the rescope flag)
```

These should already hold from the first PR of the stack; re-verify
here because any batch PR could have accidentally reintroduced the
old commands (e.g., by copying a stale CLAUDE.md snippet without
rewriting it).

## PR body template

Include in the cleanup PR body:

```
## Migration cleanup — final PR

Baseline (pre-migration) sf ai skills check findings: N
Post-migration findings: M  (Δ: M - N)

Deleted:
- .ai/commands/ (X files total)
- .ai/context/ (Y files total)

Reference updates:
- <list of CLAUDE.md / .claude/agents / .claude/commands files updated>

.cursor/ reconciliation:
- <output of sf ai skills bridge>

Untouched (confirmed):
- .ai/review/, .ai/casper-tasks/, .ai/mcp/, .ai/plans/,
  .ai/README.md, .ai/OWNERS.yml
```
