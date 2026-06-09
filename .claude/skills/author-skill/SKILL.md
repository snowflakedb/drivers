---
name: author-skill
description: Authors, modifies, or configures a Claude Code skill. Use when the user says "create a skill", "new skill", "write a skill", "my skill isn't triggering", "fix this skill", "add a capability to my skill", or "make this command-only". Not for evals.
argument-hint: "[describe what you want to create, fix, or configure]"
allowed-tools: [Bash, Read, Write, Edit, Glob, Grep]
---

# author-skill

Help the user create a new skill, modify an existing one, or configure skill frontmatter behavior. Delegates the mechanical SKILL.md-authoring work to `sf ai skills author` (CLI); handles the conversational work (interviewing the user, reviewing the generated output, iterating) in the agent window.

## Pre-flight

Before reading any reference file, answer these two questions from the user's message:

**Q1. Does a skill already exist?**
- No (user is building something new) → **CREATE**
- Yes (user has an existing skill) → continue to Q2

**Q2. What does the user want to do with the existing skill?**
- Any change to behavior — broken, wrong output, adding capability, new trigger → **MODIFY**
- Changing `disable-model-invocation`, `user-invocable`, `paths`, or `argument-hint` → **CONFIGURE**
- Reviewing or regenerating eval prompts → stop and tell the user to use the `generate-eval-set` skill instead. That's its job, not ours.

If Q2 is still ambiguous, ask one question before reading any file:
> "Are you looking to change the skill's behavior, or change how it's invoked (frontmatter flags)?"

Never default to CREATE when an existing skill is mentioned — CREATE writes a fresh SKILL.md and will overwrite.

## Proportionality

Match investigation depth to request complexity. If the user names a specific skill and a specific problem, go directly to that file — do not run broad discovery (Glob, Grep, Explore agents) when the target is already known. Reserve multi-file investigation for requests where the skill name or location is ambiguous.

## Read the reference file

Once the mode is resolved, read the corresponding file **before writing any response to the user**:

| Mode | File |
|---|---|
| CREATE | `references/create-mode.md` |
| MODIFY | `references/modify-mode.md` |
| CONFIGURE | `references/configure-mode.md` |

Do not proceed from prior context — read the file for the identified mode. Each file has its own workflow, lint gates, and stopping points.

---

## Description Accuracy Loop

The description field determines whether a skill triggers at all. Apply this loop whenever writing or patching a description — in Create or Modify.

**Structure every description as three parts:**
```
[What it does — 1 sentence, third person]
[When to trigger — 5+ specific phrases matching real user language]
[Additional contexts — edge cases that should still trigger]
```

**Calibration — vague descriptions silently fail:**

| Description | Problem | Est. activation |
|---|---|---|
| "Helps with commits" | No trigger signal | ~20% |
| "I can help you write commit messages" | First person | low |
| "A sophisticated commit message system" | Marketing speak, no phrases | low |
| "Generates commit messages. Use when the user says 'commit', 'write a commit message', 'what should my commit say'." | Specific phrases, third person | high |

**Symptom-to-fix table:**

| Symptom | Fix |
|---|---|
| Skill doesn't activate on expected phrases (trigger gap) | Add the exact failing phrase verbatim to the description — do not rephrase or generalize |
| Skill activates when it shouldn't (false positive) | Add specificity or narrow scope in the description body |
| Skill triggers but output is wrong or inconsistent | Improve instructions, add examples, add scripts for fragile steps |
| Two skills both trigger on the same prompt | Tighten the description of the one that shouldn't fire; add a negative context line |

**Iteration loop:**
1. Write or patch the description
2. Re-test with the specific failing prompt in a fresh context
3. If it still fails, apply the symptom-to-fix table and repeat
4. Stop when all reported failures are resolved

---

## Output format

This skill does not emit a fixed output block. Each mode's reference file describes what the deliverable is (a new SKILL.md written via CLI, a patch to an existing file, a frontmatter change) and ends with an explicit stopping point for user review.

Always end a session with:

1. The mode that ran (CREATE / MODIFY / CONFIGURE).
2. The file(s) changed — repo-relative paths.
3. Any follow-up command the user should run, typically:
   - `sf ai skills check <skill-dir>` — validate the written file
   - `sf ai skills generate-eval <skill-dir>` — generate or update the routing-accuracy eval set (separate skill; we do not do this work)

## Quality rules

- **The description is the most failure-prone field.** If the user reports a triggering problem, start by rewriting the description, not the body.
- **Never silently overwrite** — in MODIFY mode, summarize what changed and wait for confirmation before writing.
- **Lint gates block progression** — each mode's reference file has 🔴 checks that must pass before closing. Do not bypass.
- **Do not author the eval set here.** That's the `generate-eval-set` skill's job; our scope stops at the SKILL.md itself.

## Gotchas

- **CLI writes overwrite.** `sf ai skills author` with an intent replaces the SKILL.md on success. If the user wants to preserve the current file, diff first (`git diff`) or copy it aside before running.
- **Mode detection from path:** the CLI auto-detects CREATE vs MODIFY from whether SKILL.md exists at the target. If the user wants to regenerate from scratch on top of an existing skill, pass `--mode=create` explicitly.
- **MODIFY overwrites on success.** `sf ai skills author --mode=modify` re-reads the file each invocation, so you can iterate with `--feedback`. If you want to preserve the current file, run `git diff` before each invocation.
- **Skill names are structural.** Lowercase kebab-case, no underscores, max 64 chars, must match the directory basename. The CLI validates this; the orchestrator should also refuse when the user proposes a malformed name.

## Out of scope

- **Eval authoring.** Use the `generate-eval-set` skill — it has its own CLI (`sf ai skills generate-eval`) and its own user-review flow.
- **Running evals.** The `sf ai skills eval` command executes eval sets against Claude; that's a separate skill.
- **Creating the repo-level infrastructure** (precommit, CI, meta-skill). That's `sf ai skills repo-setup` — a one-time onboarding action.
- **Non-skill Claude tasks.** If the user's output-quality problem is with a workflow that isn't backed by a skill file, this skill can't help — point them at the skill for that specific task area.

## Examples

**Example 1 — CREATE a new skill.**
User: "I want a skill that helps me draft release notes from a PR diff."
→ Route CREATE. Read `references/create-mode.md`. Interview user for trigger phrases and tasks. Call `sf ai skills author .claude/skills/draft-release-notes --intent "..."`. Parse Output block. Review generated file with user. Optionally run `sf ai skills generate-eval` as a follow-up.

**Example 2 — MODIFY: fix a trigger gap.**
User: "my write-tests skill doesn't fire when I say 'add coverage'."
→ Route MODIFY → repair path. Read `references/modify-mode.md`. Lint-check the existing skill; add the exact phrase "add coverage" to its description frontmatter via Edit; re-run `sf ai skills check`. Re-test in a fresh session with "add coverage" and confirm it fires.

**Example 3 — CONFIGURE: hide a skill from auto-invocation.**
User: "the merge-pr skill keeps firing when I'm just asking about PRs."
→ Route CONFIGURE. Read `references/configure-mode.md`. Propose `disable-model-invocation: true` on merge-pr's frontmatter. Present summary, get confirmation, apply via Edit. Run `sf ai skills check` to confirm the frontmatter still passes lint.
