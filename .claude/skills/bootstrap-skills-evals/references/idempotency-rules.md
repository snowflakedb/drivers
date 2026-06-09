# Idempotency rules — detect-then-skip

`sf ai skills repo-setup` promises to be safe to run multiple times
(inherited from the existing stub's `Long` text). This reference
spells out the exact detection + skip pattern for every artifact
the bootstrap installs.

## Pattern

For each artifact:

1. **Detect** — is it already present?
2. **If present** — compare content to what we'd install.
3. **Decide**:
   - Byte-identical → **skip** (no-op; call it out in the
     proposal as "already installed, skipping").
   - Diverged → **ask user**. Show existing vs proposed.
     User picks overwrite / keep existing / merge.
   - Never silent-overwrite.
4. **If absent** — install.

## Per-artifact table

| Artifact | Detection | Skip condition | Ask condition |
| --- | --- | --- | --- |
| `author-skill` orchestrator | `.claude/skills/author-skill/SKILL.md` exists | Byte-identical to snowdev source | Diverged |
| `generate-eval-set` orchestrator | `.claude/skills/generate-eval-set/SKILL.md` exists | Byte-identical | Diverged |
| `run-skill-evals` orchestrator | `.claude/skills/run-skill-evals/SKILL.md` exists | Byte-identical | Diverged |
| `migrate-repo-to-skills` orchestrator | `.claude/skills/migrate-repo-to-skills/SKILL.md` exists | Byte-identical | Diverged |
| `configure-skill-settings` audit skill | `.claude/skills/configure-skill-settings/SKILL.md` exists | Byte-identical (include `eval_sets/routing-accuracy.yaml`) | Diverged |
| Precommit hook `sf-ai-skills-check` | `id: sf-ai-skills-check` present in `.pre-commit-config.yaml` | Present (text match on `id:`) | Never — additive only |
| Precommit hook `sf-ai-skills-bridge` | `id: sf-ai-skills-bridge` present | Present | Never — additive only |
| Wrapper script `sf-ai-skills-check.sh` | `.pre-commit-scripts/sf-ai-skills-check.sh` exists | Byte-identical | Diverged |
| Wrapper script `sf-ai-skills-bridge.sh` | `.pre-commit-scripts/sf-ai-skills-bridge.sh` exists | Byte-identical | Diverged |
| Buildkite step `skill_check.yml` | `.buildkite/steps/skill_check.yml` exists | Byte-identical | Diverged — always ask, because CI steps are often repo-tuned |
| Buildkite step `skill_eval.yml` | `.buildkite/steps/skill_eval.yml` exists | Byte-identical | Diverged |
| GH Actions `skill-check.yml` | `.github/workflows/skill-check.yml` exists | Byte-identical | Diverged |
| GH Actions `skill-eval.yml` | `.github/workflows/skill-eval.yml` exists | Byte-identical | Diverged |
| Lifecycle README | `.claude/skills/README.md` exists | Byte-identical | Diverged — always ask for any non-empty README |
| CLAUDE.md snippet | Root CLAUDE.md contains `sf ai skills` OR links to `.claude/skills/README.md` | Present | Never — don't add duplicate section |
| Telemetry hook (Claude) | `.claude/settings.json` exists; check `hooks.UserPromptSubmit` and `hooks.PostToolUse` for snowdev's `sf ai __hook --ide claude` command strings | Both entries already present (byte-match) | Diverged — **append-merge** (preserve adopter `permissions` and other entries) |
| Telemetry hook (Cursor) | `.cursor/hooks.json` exists; check `hooks.beforeSubmitPrompt` for snowdev's `sf ai __hook --ide cursor` command string | Entry already present (byte-match) | Diverged — **append-merge** (preserve other entries) |

## Skills with subdir content (`metadata/`, `references/`, `eval_sets/`)

For each meta-skill, check the whole subdirectory tree, not just
`SKILL.md`. Orchestrators typically ship `metadata/` and
`references/`; `configure-skill-settings` additionally ships
`eval_sets/routing-accuracy.yaml` (orchestrators don't — they're
eval-exempt).

- All files in the target match snowdev source, byte-for-byte →
  skip.
- `SKILL.md` identical but `references/*.md` diverged → ask
  per-file.
- `SKILL.md` diverged → ask at the skill level, don't drill down.

Simple implementation: diff the entire snowdev-source directory
against the target directory. If diff is empty → skip. Otherwise
render the diff and ask.

## Reporting in the proposal

Phase 3 proposal lists every artifact with its action:

```
META-SKILLS (orchestrators + audit):
  ✓ .claude/skills/author-skill/                  already installed — skip
  ✓ .claude/skills/generate-eval-set/             already installed — skip
  ⚠ .claude/skills/run-skill-evals/               DIVERGED — user decision needed
  + .claude/skills/migrate-repo-to-skills/        will copy from snowdev source
  + .claude/skills/configure-skill-settings/      will copy from snowdev source (includes eval_sets/)

PRECOMMIT HOOKS:
  ✓ sf-ai-skills-check (in .pre-commit-config.yaml)
  + sf-ai-skills-bridge (will add)

CI STEPS (Buildkite detected):
  + .buildkite/steps/skill_check.yml
  + .buildkite/steps/skill_eval.yml

README + CLAUDE.md:
  + .claude/skills/README.md
  + Root CLAUDE.md: add "Skills & Evals" section after line 8

TELEMETRY HOOKS:
  + .claude/settings.json (will create — file missing)
  ⚠ .cursor/hooks.json — APPEND-MERGE (adopter has 1 existing entry)
```

Use glyphs: `✓` = skip, `+` = will create, `⚠` = user decision
needed.

## Why some artifacts never "ask"

Two categories are additive-only (always skip if present, never
overwrite):

- **Precommit hook entries** — the `id:` line in
  `.pre-commit-config.yaml` is the identity. If the id is there,
  the hook exists. The surrounding YAML may differ (name, entry,
  args), but re-proposing a diverged version risks corrupting a
  repo-tuned setup.
- **CLAUDE.md snippet** — once the phrase `sf ai skills` is in
  the root CLAUDE.md, the bootstrap considers the pointer
  installed. Re-adding the snippet even with slightly different
  wording creates duplicate sections.

For these, the bootstrap leaves existing content alone and simply
reports "already pointed to".

## Telemetry hooks: append-merge, never overwrite

Telemetry hooks (`.claude/settings.json` `hooks` block,
`.cursor/hooks.json`) are a third category that behaves
differently from both the "ask on diverge" pattern and the
"additive-only" precommit pattern:

- The hook **commands** (`sf ai __hook --ide <ide> --hook-type
  <type>`) are spec, not negotiable. Adopters MUST NOT modify
  them; doing so detaches their telemetry from the convention.
- The **file shape** around the commands is repo-specific and
  MUST be preserved: adopter `permissions` blocks, custom
  `hooks.UserPromptSubmit` entries, top-level keys, etc.
- "Diverged" therefore means **append-merge**, not "ask the
  user". The bootstrap merges snowdev's command entries into the
  appropriate arrays and leaves everything else untouched.
- The skip case is byte-match on the snowdev command strings —
  same idempotency contract as the other artifacts, but on a
  per-entry basis instead of per-file.

See `telemetry-hooks.md` for the full requirements + detection
states + merge semantics.

## Never touched

Regardless of state, the bootstrap never modifies:

- `.ai/**` (except the review dir? — actually, `.ai/` is fully
  out of scope for `repo-setup`; it's `migrate-repo-to-skills`'s
  concern).
- Files outside the artifacts listed above.
- Existing repo content that happens to mention `sf ai skills`
  but isn't the CLAUDE.md pointer or the `.claude/skills/README.md`
  — we don't search-and-replace user prose.
