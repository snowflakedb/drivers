---
name: run-skill-evals
description: Runs routing-accuracy evals via `sf ai skills eval`. Use when the user says "run skill evals", "eval my skills", "test skill routing", "check if my skill triggers", or "run evals on <skill>". Not for authoring or eval-set generation.
argument-hint: "[skill-name or directory path]"
allowed-tools: [Bash, Read]
---

# run-skill-evals

Run routing-accuracy evals for one or more skills and summarize the results. The CLI (`sf ai skills eval`) does all the work — loads each skill's eval set, runs every prompt through Claude, scores responses, aggregates per-skill verdicts. This skill just resolves the `sf` binary, invokes the CLI, and presents the output.

Eval is **non-iterative** — unlike `generate-eval-set` (which loops on `--feedback`) or `author-skill` (which iterates on skill content), eval runs once and reports. If results show a skill failing, route the user to `author-skill` with `--mode=modify` to fix the description, and then re-run evals. Don't re-run the same eval hoping for different results; it's mostly deterministic at the contract's thresholds.

Execute the following steps in order.

## Workflow

1. **Resolve the `sf` binary.** Load `metadata/parameters.md` and follow its instructions — either `RUN_SKILL_EVALS_SF_LOCATION` is set, or `sf` on PATH is fine.

2. **Resolve the argument.** The optional argument is either:
   - A skill name (matches `--skill=<name>`), or
   - A directory path under which skills live (positional arg).

   Heuristic: if the argument exists as a directory on disk, treat it as a path. Otherwise assume it's a skill name and pass `--skill=<arg>`. If no argument is given, run against the current repo root.

3. **Invoke the CLI.** Example invocations:

   ```
   <sf> ai skills eval                    # full repo
   <sf> ai skills eval --skill=my-skill   # single skill
   <sf> ai skills eval path/to/module     # scoped to a subdirectory
   ```

   Capture the JSON output (the CLI auto-emits JSON when piped, which is how the Bash tool reads it).

4. **Parse the JSON.** The top-level shape is:

   ```
   {
     "contract_version": "...",
     "mode": "all" | "changed" | "single",
     "results": [
       { "skill_name": "...", "score": 0.93, "verdict": "PASS",
         "total": 14, "passed": 13, "failed": 1,
         "failures": [{...}] }
     ],
     "summary": { "total": N, "passed": X, "warned": Y, "failed": Z,
                  "exempt": E, "skipped": S }
   }
   ```

   `verdict` is one of `PASS` / `WARN` / `FAIL` / `EXEMPT` / `SKIPPED`. Each `failures[]` entry carries `prompt`, `expected`, `selected`, `reasoning`, and `expected_bucket` so you can explain *why* a prompt failed without re-running it.

5. **Present the summary to the user.** At minimum show:
   - One-line summary: "X passed, Y warned, Z failed, N exempt, M skipped."
   - For each WARN/FAIL skill: name, score, and the first 1–3 failed prompts with their `selected` + `reasoning`.
   - Link back to the CLI for fuller detail: "Run `<sf> ai skills eval --skill=<name>` for full detail."

6. **Offer follow-up actions** based on the result:
   - Any FAIL/WARN skills → "Would you like to fix one of these? Invoke `author-skill` with `--mode=modify --feedback="..."` to patch the description." (A description change is the typical fix for trigger-gap / false-positive failures.)
   - All PASS → confirm and exit. Optionally suggest `<sf> ai skills status` to see coverage across the repo.

## Output format

No fixed output block — presentation adapts to the run. Always end with:

1. The one-line verdict summary.
2. The exact CLI command run (for reproducibility).
3. One follow-up suggestion (fix a failure, re-run changed-only, generate missing eval sets).

## Quality rules

- **Never re-run an identical eval hoping for different results.** Verdicts are deterministic at the contract's thresholds; a WARN doesn't flip to PASS on a second run. Only re-run after a real change to the skill or its eval set.
- **Show failed prompts verbatim.** The eval's value is letting a developer see exactly which prompts missed and what Claude picked instead. Never summarize failures away — copy the `prompt`, `selected`, and `reasoning` fields as-is.
- **Respect EXEMPT and SKIPPED.** Skills with `disable-model-invocation: true` are contractually exempt; skills without an eval set are skipped. Don't treat either as a failure.
- **If the user asked about a specific skill, prefer `--skill=<name>`** over a full-repo run — full-repo runs can take minutes; single-skill runs finish in 10–60 seconds.

## Gotchas

- **Eval runs are expensive.** Each prompt = one `sf ai claude -p` call (~3–8s). A 20-prompt skill takes ~1–3 minutes with the default `--parallelism=4`. Warn the user before running a full-repo eval on a large codebase.
- **CI output is intentionally different.** `--context=ci` applies the contract's enforcement; `--context=local` (default) is always advisory. If the user says "run it like CI does", add `--context=ci` to the invocation.
- **`--max-prompts` exists for debugging.** If a user wants to preview an eval without running the full set, pass `--max-prompts=5`. Ratio-preserving truncation keeps should_trigger/should_not_trigger balanced.
- **The CLI's `--format=pretty` is for humans.** When the Bash tool pipes CLI output into this skill, it gets JSON automatically. Don't pass `--format=pretty` in skill invocations — JSON is what you want to parse.
- **Every-prompt parse failure exits non-zero.** If every prompt in the run comes back unparseable (no `SELECTED_SKILL:` line), the CLI exits 1 regardless of enforcement level. This is a run-integrity signal: the eval produced no usable data, typically because `sf ai claude` is misconfigured (wrong auth, wrong claude version, network error). Re-run with `sf --log-level=debug ai skills eval …` to see the argv the Invoker composed and the raw Claude stdout — that's usually enough to tell whether the response was prose, an error page, or an agent-mode transcript. A partial parse-error (some pass, some fail to parse) does NOT trigger this gate — those flow through normal scoring.

## Out of scope

- **Authoring or modifying skills.** Use `author-skill` for CREATE or MODIFY. This skill only runs evals; it doesn't fix them.
- **Generating eval sets.** Use `generate-eval-set` for that.
- **Running evals in CI.** CI runs a repo-specific eval driver script that calls `claude -p` directly with Cortex env vars (`ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN`) — see `bootstrap-skills-evals/references/ci-<flavor>.md` → "Why a script, not `sf ai skills eval`?". `sf ai skills eval` itself works fully on cloud workspaces (where this orchestrator skill runs); on CI workers `sf ai claude` isn't provisioned, so the CLI returns parse_error on every prompt. This orchestrator skill is for interactive local use only.

## Examples

**Example 1 — run evals on a single skill.**
User: "run evals on release-notes"
→ Invoke `<sf> ai skills eval --skill=release-notes`. Parse JSON. Show: "release-notes: 19/20 passed, 1 failed (WARN). Failed prompt: `{prompt}` → Claude picked `{selected}` — reasoning: {reasoning}." Offer: "Want to fix? Run `author-skill` with --mode=modify and a feedback hint."

**Example 2 — full-repo run.**
User: "run skill evals"
→ Warn about expected duration ("~5–10 minutes for 8 skills with default parallelism"). Invoke `<sf> ai skills eval`. Show summary table. Drill into any FAIL/WARN skills.

**Example 3 — scope to a subdirectory.**
User: "eval the skills in ai-devprod/"
→ Invoke `<sf> ai skills eval ai-devprod/`. Same output shape, scoped to that subtree.

**Example 4 — all passing.**
User: "test skill routing"
→ Full-repo run. If every skill passes, say so cleanly: "All N skills PASS. Contract thresholds: PASS ≥0.80, WARN ≥0.60 (see `<sf> ai skills contract`)."
