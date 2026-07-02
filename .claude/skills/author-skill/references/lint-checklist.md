# Lint Checklist

Run `sf ai skills check <skill-dir>` to cover most of this
automatically — the CLI's deterministic rules cover almost every
check below. This reference lists the full set so you can diagnose
issues by hand when the CLI flags them.

Fix all 🔴 issues before continuing past any lint gate.

---

## Frontmatter Validity

| Check | Severity |
|---|---|
| `name` field exists | 🔴 |
| `name` exactly matches directory name | 🔴 — silent routing failure otherwise |
| `name` is lowercase kebab-case (no underscores, uppercase, spaces) | 🔴 |
| `name` is max 64 chars | 🔴 |
| `name` does not contain "anthropic" or "claude" | 🟡 |
| `description` field exists and is non-empty | 🔴 |
| `description` is under 1,024 chars | 🔴 |
| First 250 chars of `description` stand alone as a complete trigger signal | 🔴 |
| `description` contains 5+ trigger phrases | 🔴 |
| `description` is third person (no "I help", "I will") | 🟡 |
| `description` contains no XML tags | 🟡 |
| `description` does not repeat the `name` verbatim | 🟡 |
| `paths` globs are syntactically valid (if present) | 🔴 |
| `user-invocable: false` + `disable-model-invocation: true` not both set | 🔴 — nothing can ever invoke the skill |

## Placement

| Check | Severity |
|---|---|
| If skill body or description names a specific module/subsystem, skill is placed at that module's `.claude/skills/` (not repo root) | 🔴 — wrong placement causes silent load failures |
| Repo-root placement is explicitly justified (skill is genuinely cross-cutting) | 🔴 — unjustified root placement is structurally hard to reverse |

## File and Path Existence

| Check | Severity |
|---|---|
| `eval_sets/routing-accuracy.yaml` exists (regenerate via `sf ai skills generate-eval` whenever the skill's description or trigger surface changes) | 🔴 |
| Every file path mentioned in SKILL.md body exists on disk | 🔴 |
| Every `scripts/` reference exists | 🔴 |
| Every `references/` file exists | 🔴 |
| No chained references (ref file links to another ref file) | 🔴 |

## SKILL.md Structure

| Check | Severity |
|---|---|
| All required sections present: Opening, Workflow, Output Format, Quality Rules, Gotchas, Out of Scope, Examples | 🔴 |
| No broken markdown links | 🔴 |
| SKILL.md is under 500 lines | 🟡 |
| Reference files >100 lines have a table of contents | 🟡 |

## Output format

When running `sf ai skills check <skill-dir>`, the CLI emits:

```
<skill-dir>/SKILL.md

🔴  name matches directory name                           ✓
🔴  description first 250 chars stand alone               ✓
🔴  all body sections present                             ✗  Missing: Out of Scope
🔴  all referenced file paths exist                       ✓
🔴  description ≥ 5 trigger phrases                       ✗  Found: 3
🟡  description is third person                           ✓

2 critical issues must be fixed before running evals.
```

When running the CLI surfaces `✗` findings, consult the Hint on each
finding for the concrete fix.
