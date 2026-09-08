---
name: behavior-compatibility-review
description: >
  Reviews driver changes against old/legacy behavior for BDs, implementation
  gaps, and test gaps. Run /behavior-compatibility-review before pushing JDBC,
  Python, ODBC, Node.js, or sf_core changes. Not suite mapping, Jira bug
  coverage, or line coverage.
argument-hint: "[optional: staged | range]"
disable-model-invocation: true
allowed-tools: [Bash, Read, Glob, Grep]
---

# Behavior compatibility review

Diff-scoped self-review for any Universal Driver wrapper. Lives at repo-root
`.claude/skills/` because JDBC, Python, ODBC, Node.js, and shared `sf_core`
share one workflow.

Read `references/driver-evidence.md` for BD files, coverage indexes, and
old-contract locations.

Invocation is slash-command only while the pilot measures finding quality.
Do not call this from `end-of-task-self-review` until that follow-up is
approved.

## Workflow

1. **Range.** Default:
   - `base=$(git merge-base origin/main HEAD)`
   - inspect `git diff "$base"` for committed, staged, and unstaged changes
   - inspect `git ls-files --others --exclude-standard` for untracked files
   The review input is the union of both results. Read every relevant untracked
   production/test file even when `git diff "$base"` is empty.
   Override only if the user asks: staged only (`git diff --cached`), or an
   explicit `base..HEAD`.
2. **Classify.** Map changed paths to drivers using the evidence table.
   Docs-only / changelog-only / skill-docs-only → empty report and stop.
3. **Surfaces.** From the production and test diff, list public APIs,
   connection/statement options, error codes, wire/login fields, type
   conversions, and connect/execute/result paths.
4. **Lookup (those surfaces only).** Use only the classified drivers' evidence
   paths: matching `BehaviorDifferences.yaml` entries, that driver's coverage
   index when it exists, its in-tree old snapshot when present, and existing UD
   tests. For `sf_core/` diffs, look up **all** wrappers for the shared contract,
   not only wrappers also in the diff.
5. **Compare assertions**, not filenames: same option, error code, boundary
   value, format token, error polarity.
6. **Relevance gate.** Drop findings that would exist even if this diff were
   empty (known unfinished APIs). Exception: this diff implements, stubs,
   skips, or documents that surface.
7. **Report and exit.** Read-only. Do not file Jira, edit BD YAML, add tests,
   or trim `_old-driver-reference/` during this invocation. Those actions
   require a separate request after this review has ended.

## Output format

```text
Behavior compatibility review
Range: <merge-base> … worktree
Drivers: <list>
Surfaces: <list>

Findings (N)
1. [<severity>] <kind>
   Surface: …
   Diff: <paths>
   Old evidence: …
   New evidence: …
   BD: BD#N | none
   Suggested action: …

Already tracked: BD#…
No finding on: …
Not in scope: unmapped old tests for APIs not in this diff.
```

Empty findings is success: write `Findings (0)` and stop. Do not pad with
suite-wide “not implemented” items.

**Finding kinds:** `unintended_bd` | `undocumented_known_bd` | `impl_gap` |
`test_gap` | `weak_test` | `insufficient_evidence`

`already_tracked` is not a finding kind. Put matching existing BDs only in the
separate `Already tracked` section so they do not inflate `Findings (N)`.

**Severity:** `blocker` (customer-visible break on this path) |
`should-fix-in-pr` | `note`

**Suggested action** is advice only (fix code, add a test via `add-tests`,
or add a BD entry). Do not apply it in this run.

## Quality rules

- Compare observable old-driver vs new-driver behavior.
- A similarly named test is not coverage without the same assertions.
- Gherkin under `tests/definitions/` is intent only, never proof.
- Wrapper-only APIs need wrapper tests; `sf_core` unit tests can support a
  finding but do not replace them.
- If suggesting a future BD, use “new driver” / “old driver” prose. Never
  invent `SNOW-` keys. Leave `reviewed: false` if a BD is later added.
- Missing old source → `insufficient_evidence`, not a fabricated BD.

## Gotchas

- `sf_core` login/option/type changes can affect every wrapper even when the
  diff has no `jdbc/` / `python/` / `odbc/` / `nodejs/` / `dotnet/` files.
  Still review all five wrappers against the shared surface.
- `.NET` has a coverage index and no BD file; say so and continue.
- Node’s `_old-driver-reference/` is a frozen snapshot. Other drivers often
  have only the YAML index in this repo.
- An empty `ud_tests` list is not itself a finding. It matters only when the
  row matches a surface changed by this diff and the observable contract needs
  wrapper coverage.
- If a non-doc path is not classified by the evidence table, name it in the
  report and determine whether it changes a wrapper-visible shared contract.
  Review all wrappers when it does; otherwise return `insufficient_evidence`
  rather than silently skipping it or scanning every coverage index.
- Do not treat skipped `it.todo` / `@Disabled` as coverage (`weak_test` or
  `test_gap`).
- Neighbor skills: suite YAML mapping → `test-coverage-mapper`; one Jira
  bug → `bug-regression-coverage`; JDBC flakiness → `jdbc-test-reviewer`;
  general pre-PR hygiene → `end-of-task-self-review`.

## Out of Scope

- Inventorying unmapped old tests for APIs this diff does not touch.
- Filing Jira (including epic SNOW-4073010) on this invocation.
- Writing or editing `BehaviorDifferences.yaml`, coverage YAML, or tests
  as part of the default run.
- Trimming `nodejs/tests/_old-driver-reference/`.
- Line/branch/jacoco coverage percentages.
- Auto-invoking from `end-of-task-self-review` (follow-up after quality is
  proved).

For pilot instructions, feedback fields, and the quality gate for enabling
automatic invocation later, read `references/nodejs-pilot.md`.

## Examples

**JDBC option parsing (should find).** Diff tightens JDBC connection-property
validation with no new test. Old coverage YAML / BD describe looser
acceptance. Report `test_gap` or `unintended_bd` on that property only.

**Python restates a documented BD (already tracked).** Diff matches an
existing `python/BehaviorDifferences.yaml` row. Put the BD id under
`Already tracked`; do not open a finding for the same contract.

**Node docs-only.** Diff is README / changelog. Empty report.

**sf_core shared login payload.** Diff omits a login JSON field in
`sf_core`. Review JDBC, Python, ODBC, Node.js, and .NET call sites and tests
for that field, even if those trees are unchanged in the diff.
