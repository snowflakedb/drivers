# Modify Mode

Targeted changes to an existing skill. Covers both repair (something
broken) and addition (new capability). Never rewrites passing sections.

The mechanical SKILL.md editing is delegated to
`sf ai skills author --mode=modify`; this reference guides the
conversational work around it.

## Workflow overview

1. Read the existing skill in full
2. Classify the change (repair vs addition)
3. Gather the specific change ask from the user
4. Resolve the `sf` binary path
5. Invoke `sf ai skills author --mode=modify`
6. Review the generated diff with the user
7. Re-lint with `sf ai skills check`
8. Hand off to `generate-eval-set` for eval set updates
9. Fresh-context test

---

## Step 1: Read the existing skill in full

Understand what currently passes before touching anything. Read the
full SKILL.md plus any files under `references/` and `scripts/`.

---

## Step 2: Classify the change

Determine from context which path applies. If unclear after reading
the user's message, ask one question:

> "Is this fixing something that's broken, or adding something that
> was never there?"

| Answer | Path |
|---|---|
| Something is broken, stopped working, never triggered correctly, wrong output | **Repair** |
| New capability, new trigger phrase, new output format, deliberate expansion | **Addition** |

| Category | Typical symptom | What the CLI prompt does |
|---|---|---|
| Trigger gap | Skill doesn't activate on expected phrases | Adds the failing phrase verbatim to description |
| False positive | Skill activates when it shouldn't | Adds specificity / narrows scope in description |
| Output quality | Skill triggers but produces wrong or inconsistent output | Updates body sections (workflow, output format) |
| Broken reference | Script, path, or file reference no longer exists | Updates the path or removes the stale reference |
| Coexistence conflict | Two skills both trigger on the same prompt | Tightens the description of the one that shouldn't fire |

---

## Step 3: Gather the specific change ask

`sf ai skills author --mode=modify` takes at least one of `--intent`
or `--feedback`:

- `--intent` — broad description of the change ("expand this skill to
  also handle monorepo tests")
- `--feedback` — surgical, specific change ("add the trigger phrase
  'add coverage' verbatim to the description")

Prefer `--feedback` when the user has a specific, concrete ask. Use
`--intent` only for broader scope expansions.

Before invoking, get the user to state the change clearly. For trigger
gaps, capture the **exact failing phrase verbatim** — do not rephrase
it. For false positives, capture a representative prompt that
incorrectly triggers.

---

## Step 4: Resolve the `sf` binary

Read `metadata/parameters.md` and follow its instructions to resolve
the `sf` command location. Use this `sf` for all subsequent invocations.

---

## Step 5: Invoke `sf ai skills author --mode=modify`

Run:

```
<sf> ai skills author <skill-dir> --mode=modify --feedback "<specific change>"
```

(or `--feedback-file <path>` for long / multi-line asks, or
`--intent "..."` for broader expansions.)

Parse the structured Output block:

- `actions taken:` — the bullet list. Look for:
  - `Classified change: <repair | addition | mixed>`
  - `Sections modified: <comma list>`
- `new status: <icon>` — one of ✅ / ❌
- `stop: <reason>` — present only on failure
- `skill_md:` — SKILL.md body, indented; emitted only on failure (last
  attempted content, for debugging)

Based on `new status:`:

- ✅ — success. The SKILL.md was overwritten. Continue to Step 6.
- ❌ — failure. Show `stop:` to the user.
  - If `stop:` mentions `existing SKILL.md` missing, the user pointed
    at the wrong path. Confirm the skill location and retry.
  - If `stop:` mentions `Claude refused`, the ask was too ambiguous.
    Rephrase with more specificity and retry.
  - If `stop:` mentions `did not validate after N attempts`, the
    `skill_md:` body in the Output block has the last attempt for
    debugging. Either fix the issue manually via Edit, or retry with
    a sharper `--feedback`.

### Iterating with `--feedback`

If the modified SKILL.md still isn't quite right, re-run with a
refined `--feedback` ask:

```
<sf> ai skills author <skill-dir> --mode=modify --feedback "<refined change>"
```

The CLI re-reads the (now-modified) file and patches it again.

---

## Step 6: Review the generated diff with the user

Run `git diff <skill-dir>/SKILL.md` to show the user the exact change.
Walk through:

- Does the description change preserve third-person voice and 5+
  trigger phrases?
- For repairs: does the exact failing phrase appear verbatim in the
  description?
- For additions: are new trigger phrases in the description (not just
  in a body section)?
- Are any passing sections accidentally rewritten?

If the user wants changes, iterate via `--feedback` (Step 5).

---

## Step 7: Re-lint

Run:

```
<sf> ai skills check <skill-dir>
```

Fix any 🔴 issues before proceeding. The CLI's author command
enforces a subset of rules at generation time; running `check`
afterward catches anything downstream.

---

## Step 8: Update the eval set

**Never edit `eval_sets/routing-accuracy.yaml` directly.** Always run
`sf ai skills generate-eval <skill-dir>` (or invoke the
`generate-eval-set` skill) after a MODIFY. The command handles both
creation (if missing) and updating (if existing).

---

## Step 9: Fresh-context test

Re-test the skill with a fresh context (no memory of the edit):

- **Repair**: confirm the reported failure no longer reproduces —
  feed the original failing prompt and verify the skill now triggers
  (or doesn't trigger, depending on the repair type).
- **Addition**: confirm at least one new trigger phrase fires AND at
  least two existing trigger phrases still pass.

---

## Step 10: Summarize

Summarize to the user:

- The file(s) changed (`<skill-dir>/SKILL.md`).
- The classification the CLI reported (`repair` / `addition` /
  `mixed`) and sections modified.
- Any follow-up the user should take (typically: regenerate the eval
  set via `generate-eval-set`, run fresh-context tests themselves).
