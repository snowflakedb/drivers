---
name: generate-eval-set
description: Author or update a routing-accuracy eval set for a skill. Shells out to `sf ai skills generate-eval`, presents the written file to the user, and helps them iterate with `--feedback`.
arguments: [skill-path]
argument-hint: "<path-to-SKILL.md or skill dir>"
allowed-tools: [Bash, Read]
---

# generate-eval-set

Help the user author or update a skill's routing-accuracy eval set. The CLI does the heavy lifting (prompting Claude, validating, writing the YAML); this skill is the agent-window orchestrator that presents the result and collects revision feedback when the user wants changes.

Execute the following steps in order.

1. Validate the `$skill-path` argument. If empty, ask the user which skill to generate evals for (a path to a SKILL.md or the skill's directory). Do this as a plain text question — do NOT use AskUserQuestion.

2. Load `metadata/parameters.md` and follow its instructions for resolving the `sf` command location. Use this `sf` for all subsequent invocations in this skill.

3. First invocation: run `<sf> ai skills generate-eval <skill-path>`

   The command always writes the eval set to `<skill-dir>/eval_sets/routing-accuracy.yaml`. Parse the structured output block:

   - `actions taken:` — the bullet list (or "none")
   - `new status: <icon>` — one of ✅ / ❌
   - `stop: <reason>` — present only on failure
   - `eval_yaml:` — YAML body, indented; emitted only on failure (the last-attempted content, for debugging)

4. Based on `new status:`:

   - ✅ — success. The file was written. Read `<skill-dir>/eval_sets/routing-accuracy.yaml` and show the user a brief summary (N should_trigger prompts, M should_not_trigger prompts, which sibling skills are referenced). Continue to step 5.

   - ❌ — failure. Show the `stop:` reason to the user. If the reason mentions `disable-model-invocation`, explain that routing-accuracy evals are untestable for this skill and exit. Otherwise offer to retry with `--feedback` describing what should go differently, or to exit.

5. Present the written file to the user and ask what they want to do:

   ```
   I've written the eval set to <path>. Options:
     1. Ship it as-is (nothing more to do).
     2. Revise it — tell me what to change and I'll re-run with
        --feedback.
     3. Edit it yourself — open the file, make changes, then run
        `<sf> ai skills check <skill-path>` to validate.
   ```

   Ask this as plain text — do NOT use AskUserQuestion.

6. If they pick option 2 (revise):

   a. Capture their revision ask as a string. Single-line asks go into `--feedback "<text>"`. Long or multi-line asks: write the text to a temp file and pass `--feedback-file <path>`.
   b. Re-run: `<sf> ai skills generate-eval <skill-path> --feedback "<text>"` (or `--feedback-file <path>`).
   c. The command overwrites the file on disk. Parse the output block the same way and loop back to step 4.

7. If they pick option 1 (ship as-is) or option 3 (they'll edit themselves): confirm and exit. Optionally suggest they run `<sf> ai skills status` to see updated coverage for their repo.

## Notes

- Every invocation re-reads the current file from disk, so the user can freely edit between iterations — their edits are not clobbered unless they re-run the command.
- The command is fully non-interactive. Never expect it to prompt; just parse its structured output.
- Validation retries (up to 3) are handled inside the command. If the output reports `stop: eval set did not validate after N attempts`, the `eval_yaml:` block contains the last attempt for debugging.
- `--timeout` defaults to 5 minutes; if a run times out, surface that to the user and offer to retry.
