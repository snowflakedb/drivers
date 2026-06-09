---
name: configure-skill-settings
description: >
  Audits SKILL.md frontmatter across .claude/skills/ directories — flags disable-model-invocation, paths vs globs, description length, name/directory mismatches, and skill visibility/shadowing across a subtree. Use when the user says "audit my skill settings", "review skill frontmatter", "configure skill settings", or "check my skills for frontmatter issues".
argument-hint: "<folder-path, e.g. GlobalServices/modules/metadata/>"
---

# Configure Skill Settings

Review and configure frontmatter settings across Claude Code skills in a directory tree. Interactive — never writes changes without explicit approval.

This skill does NOT invoke `sf` — it's a self-contained frontmatter audit that does its work with the Claude Code tools (no shell-out to sf-cli). Run `sf ai skills check` separately afterwards if you want a deterministic validation pass over the changes.

## Arguments

`<folder-path>` — **Required.** Path to a directory containing skills. Examples:
- `GlobalServices/.claude/skills/` — all GS-level skills
- `GlobalServices/modules/metadata/` — skills in metadata and its subdirectories
- `.claude/skills/` — root-level skills

If the user provides a module path (not a `.claude/skills/` path), search for `.claude/skills/` directories within it.

---

## Step 1: Discover Skills

Glob for `**/.claude/skills/*/SKILL.md` under the given path.

Display a summary table:

```
| # | Skill Name              | Dir                                          | Auto-Invoke | Paths | Desc (first 60 chars)         |
|---|-------------------------|----------------------------------------------|-------------|-------|-------------------------------|
| 1 | build-gs                | GlobalServices/.claude/skills/build-gs/      | yes         | 0     | Build Global Services with... |
| 2 | context-atq             | GlobalServices/modules/metadata/.claude/...  | no          | 0     | Contains context and instr... |
```

Column definitions:
- **Auto-Invoke**: "no" if `disable-model-invocation: true`, otherwise "yes"
- **Paths**: count of entries in `paths:` field (0 if absent)

Report total skill count.

---

## Step 2: Ancestor Walking Visualization

Explain how Claude Code discovers skills: it walks from the current file's directory upward to the repo root, collecting every `.claude/skills/` directory along the way. Skills from deeper directories take priority over root-level skills with the same name (shadowing).

Show the directory tree from repo root to the given folder, marking which `.claude/skills/` directories exist at each level:

```
/home/repo/snowflake/
├── .claude/skills/                          ← 36 skills (root-level)
│   ├── build-gs/                            (shadowed by GS-level copy)
│   ├── triage-ticket/
│   └── ...
├── GlobalServices/
│   ├── .claude/skills/                      ← 8 skills (GS-level)
│   │   ├── build-gs/                        (shadows root build-gs)
│   │   └── write-gs-tests/
│   └── modules/
│       └── metadata/
│           ├── .claude/skills/              ← 12 skills (metadata-level)
│           │   ├── context-md-core/
│           │   └── ...
│           └── replication-core/
│               └── .claude/skills/          ← 7 skills (replication-level)
```

Highlight:
1. **Name collisions** — skills that shadow a parent-level skill of the same name
2. **Uncovered sibling directories** — module directories at the same level that have no `.claude/skills/` directory (potential gaps in coverage)
3. **Total skill count visible** from a file in the target directory (sum of all ancestor `.claude/skills/` directories)

---

## Step 3: Review disable-model-invocation

Show which skills are command-only (`disable-model-invocation: true`) vs auto-invocable (default).

Recommend `disable-model-invocation: true` for:
- Context-loading skills (name starts with `context-` or `load-context-`)
- Dangerous/destructive operations (name starts with `deploy-`, `delete-`, `reset-`)
- Skills already marked `user-invocable: true` (redundant double-opt-in)
- Skills with very broad descriptions that may overtrigger

Recommend keeping auto-invocation for:
- Build/test/lint skills that should trigger on relevant file edits
- Diagnostic skills that should trigger on error descriptions
- Code generation skills that should trigger on "write tests for X"

Present recommendations as a batch:

```
Recommended changes:
  context-md-core:     auto-invoke → command-only  (context-loading skill)
  context-md-entity:   auto-invoke → command-only  (context-loading skill)
  deploy-temptest:     already command-only ✓

Apply disable-model-invocation: true to all 2 skills? [y/n/select individually]
```

Wait for user confirmation before proceeding.

---

## Step 4: Review paths

For each skill, show the current `paths:` value (or "none").

Suggest `paths:` additions based on:
1. **Skill location** — a skill in `GlobalServices/modules/data-lake/.claude/skills/` likely wants paths matching `GlobalServices/modules/data-lake/**`
2. **Sibling module directories** — replication skills may want `**/replication-*/**` to cover `replication-core/`, `replication-impl/`, `replication-api/`
3. **Existing patterns** — show what other skills in the same directory use for `paths:`

For each suggestion, explain the effect: "This skill will auto-load when editing files matching this pattern."

Present as a batch with the ability to accept, modify, or skip each.

---

## Step 5: Fix Issues

Scan for and offer to fix:

1. **`globs:` field present** — Cursor-only field, invalid in Claude Code. Offer to convert to equivalent `paths:` entry.

2. **Description over 250 characters** — first 250 chars matter most for auto-invoke matching. Offer to help rewrite to front-load the key use case within 250 chars while keeping the full description intact.

3. **SKILL.md over 500 lines** — suggest which sections could be extracted to `references/` files.

4. **Missing `name:` or `description:`** — offer to add based on directory name and skill body content.

5. **Name/directory mismatch** — offer to rename the `name:` field to match the directory, or rename the directory.

---

## Step 6: Preview and Apply

Show a unified diff of ALL proposed changes across all skills:

```diff
--- a/GlobalServices/.claude/skills/context-atq/SKILL.md
+++ b/GlobalServices/.claude/skills/context-atq/SKILL.md
@@ -1,5 +1,6 @@
 ---
 name: context-atq
+disable-model-invocation: true
 description: >
   Contains context and instructions for working with the Async Task Queue
```

Require explicit "yes" to apply. After applying, display a summary:

```
Applied 4 changes to 3 skills:
  context-atq:    added disable-model-invocation: true
  context-md-core: added disable-model-invocation: true
  build-gs:       added paths: ["GlobalServices/**"]
```

---

## Quality Rules

- Never modify skill body content — only frontmatter fields
- Preserve existing YAML formatting style (quoted vs unquoted strings, flow vs block sequences)
- Do not add fields the user did not request or approve
- Do not remove existing fields without explicit request
- Always show the diff before writing any changes
- If unsure about a recommendation, present it as a question, not a default

## Gotchas

- `globs:` is a Cursor-only field — Claude Code ignores it. Convert to `paths:` if present.
- `user-invocable: true` is the default — adding it explicitly is redundant noise
- `allowed-tools:` is Claude Code-specific and not portable to other agents
- Skills inside `references/` subdirectories are NOT discovered by Claude Code — they are reference documents, not skills
- Shadowed skills (same name at a deeper level) completely replace the parent — they do not merge

## Out of Scope

- **Skill body edits** — this skill only touches frontmatter. For body content, edit manually or use the `author-skill` skill in MODIFY mode.
- **Creating new skills** — redirect to the `author-skill` skill
- **Running validation checks** — `sf ai skills check` (precommit hook and CI step) handles automated validation
- **Generating eval sets** — redirect to the `generate-eval-set` skill
