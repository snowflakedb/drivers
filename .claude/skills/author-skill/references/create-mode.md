# Create Mode

Builds a new skill from scratch, or captures a workflow just completed
in the current session. The mechanical SKILL.md authoring is delegated
to `sf ai skills author`; this reference guides the conversational work
around it.

## Workflow overview

1. Determine artifact type
2. Determine placement
3. Interview the user (short — most answers become the `--intent` string)
4. Resolve the `sf` binary path
5. Invoke `sf ai skills author` with the assembled intent
6. Review the generated SKILL.md with the user
7. Run `sf ai skills check` to confirm lint passes
8. Hand off to `generate-eval-set` skill for routing-accuracy evals
9. Fresh-context test

---

## Step 1: Determine artifact type

| Content type | Artifact | Key frontmatter |
|---|---|---|
| Hard constraint, applies everywhere | Rule (CLAUDE.md entry) | — |
| Hard constraint, applies to specific paths | Path-scoped rule | — |
| Procedural workflow, safe | Skill | — |
| Procedural workflow, destructive or irreversible | Skill (command-only) | `disable-model-invocation: true` |
| Orientation or architecture, <200 lines | CLAUDE.md | — |
| Domain knowledge, large or path-specific | Context skill (auto-trigger) | `user-invocable: false` + `paths:` |
| Stale, duplicate, or common knowledge | Delete | — |

**Loading behavior drives the decision:**
- CLAUDE.md and unconditional rules are always loaded — keep them small.
- Skills load on-demand when intent matches — safe for larger content.
- Context skills (`user-invocable: false` + `paths:`) auto-load on file
  match without explicit invocation.

Do not put always-true facts in a skill. Do not put workflows in a
CLAUDE.md.

---

## Step 2: Determine placement

Always the deepest applicable directory:

```
Module-specific    →  <module>/.claude/skills/
Cross-cutting      →  .claude/skills/   (rare; state the justification)
Personal           →  ~/.claude/skills/
```

Only move up when multiple modules genuinely need the same skill.
`paths:` can narrow a skill's loading after placement is correct — it
cannot expand discovery to directories beside or above the skill's
location.

**Scope detection:** does the user's message or the session context name
a specific module, component, or subsystem? If yes → propose that
module's `.claude/skills/` directory and state the reason. If no clear
signal → propose repo root, but still state the justification. Never
silently default to root.

---

## Step 3: Interview the user

**Long-session note:** this skill is typically invoked after a working
session. If earlier context is compressed, the pre-fill values below
may be incomplete. Ask the user to confirm each item explicitly rather
than assuming the proposals are correct.

**First, scan visible context** for evidence of the workflow: tool calls
executed, file paths touched, phrasings the user used, decisions made.
Present a confirmation block with pre-filled answers:

```
Based on what I can see in this session:
  Tasks:           [proposed or "not visible"]
  Trigger phrases: [proposed or "not visible"]
  Output format:   [proposed or "not visible"]
  Scope:           [proposed directory] — because [one-line justification]
  Invocation:      [auto-trigger / command-only or "not visible"]

Earlier context may be compressed — correct anything that looks wrong.

Accept each, correct it, or type "skip" to answer fresh.
```

Ask interview questions only for items the user discarded or had no
visible evidence for. Prioritize essentials first — at most 3–4
questions total.

**Essential — must resolve before invoking the CLI:**

- 2–3 concrete tasks the skill handles — defines scope
- How users phrase these requests verbatim — become trigger phrases;
  the description is built entirely from them
- Auto-trigger vs. command-only — `disable-model-invocation: true` is
  structural; default is auto-trigger

**Essential when module-specific signals are present:**

- Which directory this skill applies to — ask before invoking the CLI:
  *"This skill references [module] — should it live at
  `[module]/.claude/skills/` or does it apply across the whole repo?"*

**Deferrable — ask only if genuinely unclear:**

- Expected output format — can be sketched during generation
- Whether scripts, references, or assets are needed
- Whether it interacts with MCP tools

Ask these as plain text — do not use AskUserQuestion.

---

## Step 4: Resolve the `sf` binary

Read `metadata/parameters.md` and follow its instructions to resolve
the `sf` command location. Use this `sf` for all subsequent invocations
in this skill.

---

## Step 5: Invoke `sf ai skills author`

Assemble the `--intent` string from the interview answers. It should
include:

- 1-sentence description of what the skill does
- 2–3 concrete tasks
- 5+ verbatim trigger phrases (quoted) that users would type
- Invocation preference ("should auto-trigger" or "command-only only")
- Expected output format if known

Run:

```
<sf> ai skills author <skill-dir> --intent "<assembled intent>"
```

`<skill-dir>` must be `.claude/skills/<name>/` relative to the module
root; `<name>` is lowercase kebab-case, max 64 chars.

Parse the structured Output block:

- `actions taken:` — the bullet list (or "none")
- `new status: <icon>` — one of ✅ / ❌
- `stop: <reason>` — present only on failure
- `skill_md:` — SKILL.md body, indented; emitted only on failure (last
  attempted content, for debugging)

Based on `new status:`:

- ✅ — success. The SKILL.md was written. Continue to Step 6.
- ❌ — failure. Show `stop:` to the user. If the reason mentions
  `intent` being empty, the intent failed to parse — ask for more
  detail and retry. If the reason mentions `Claude refused`, the model
  rejected the intent — ask for a clearer description and retry.
  Otherwise offer to retry with `--feedback` describing what should go
  differently, or to exit.

### Iterating with `--feedback`

If the user reviews the generated SKILL.md and wants changes:

```
<sf> ai skills author <skill-dir> --intent "<same intent>" \
     --feedback "<revision ask>"
```

The command overwrites SKILL.md on success. Parse the output block the
same way. For long or multi-line asks, write them to a temp file and
use `--feedback-file <path>` instead.

---

## Step 6: Review the generated SKILL.md with the user

Read the written file and walk the user through:

- Description: first 250 chars stand alone? 5+ trigger phrases? Third
  person? Under 1024 chars?
- Body: are the sections present they care about (Workflow, Output
  format, Quality rules, Gotchas, Out of Scope, Examples)?
- Any obviously wrong tasks or trigger phrases?

If the user wants changes, iterate via `--feedback` (Step 5).

---

## Step 7: Lint

Run:

```
<sf> ai skills check <skill-dir>
```

Fix any 🔴 issues before proceeding. The CLI's author command enforces
a subset of lint rules at generation time; running `check` afterward
catches things like broken references to files the user later adds,
placement issues, etc. See `references/lint-checklist.md` for the full
rule list.

**⚠️ Stopping point:** present the check report to the user. Do not
proceed until all 🔴 issues are resolved.

---

## Step 8: Generate the eval set

Hand off to the `generate-eval-set` skill:

> "The SKILL.md is ready. To generate the routing-accuracy eval set,
> run `/generate-eval-set <skill-dir>` or ask me to invoke it."

Do not attempt to author `eval_sets/routing-accuracy.yaml` by hand —
that's `generate-eval-set`'s job.

---

## Step 9: Fresh-context test

Use the "Claude A / Claude B" pattern: you (Claude A) designed the skill
with the user; test it with a fresh context (Claude B) that has no
memory of the design process.

Run at least 3 prompts from the eval set — mix `should_trigger` and
`should_not_trigger`. For any that fail, apply the Description
Accuracy Loop in the skill's SKILL.md before closing.

Do not close the Create session until at least one full pass of
fresh-context testing is done.
