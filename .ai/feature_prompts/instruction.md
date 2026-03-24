Implement TIMESTAMP_NTZ type support in the Python wrapper of the Universal Driver.

## Success Criteria

- a. **Backwards compatibility** — new tests pass on the reference (old) driver. **This is the primary correctness signal.** The reference driver is the source of truth for expected behaviour; the universal driver must match it.
- b. **Working code** — new tests pass on the universal driver
- c. **Clean code** — code review agents approve: readability, naming consistency, no repetition

---

## Deliverables

Two files must be committed and treated as **permanent documentation** of the feature, not just test artefacts:

1. **Design document** — `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md`
   The authoritative specification for what TIMESTAMP_NTZ is, how it behaves, how the driver handles it, and what the tests cover and why. Written before implementation, refined throughout, committed alongside code.

2. **Gherkin feature file** — `tests/definitions/shared/types/timestamp_ntz.feature`
   The human-readable specification of the feature's observable behaviour. Treated as documentation: every scenario is a statement about what the driver guarantees. Committed alongside code.

Both documents are living — update them as new findings emerge. They must be accurate when the task is done, not just "good enough to start coding."

---

## Document Integrity Rules

These rules are non-negotiable and apply to every agent at every step. No agent — including review agents — may override them.

### Rule 1 — "Out of scope" must be declared in the design document before it is cited

If any agent claims something is out of scope during review, implementation, or any feedback loop, it **must cite the exact entry in the "Out of scope" section of `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md`** that covers the claim. Citing a section that does not exist, paraphrasing, or reasoning by analogy is not sufficient.

If the scope question is not listed in the design document:
1. It is **assumed in scope** until proven otherwise.
2. The agent must first add an explicit entry to the "Out of scope" section of the design document, stating what is being excluded and **why** (with evidence or reasoning).
3. Only after the design document is updated may the scope exclusion be used as a justification.

An agent that cites "out of scope" without a matching design document entry must be treated as if it gave no justification at all.

### Rule 2 — The design document may only grow more specific, never less

Any edit to `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` must be an **addition or a clarification**. The following edits are forbidden:

- Removing a scenario from the scenario plan
- Weakening a requirement (e.g. changing "must" to "should", removing a specific assertion, making a description vaguer)
- Adding a new "Out of scope" entry to avoid implementing something that was previously implied in scope
- Deleting confirmed evidence (file:line references, test results)
- Making any section shorter or less precise than it was before

If a reviewing agent proposes or makes such an edit, it must be reverted immediately. The edit is a red flag that the agent is attempting to reduce its own workload by lowering the bar rather than doing the work.

**When reading any agent's proposed design document change, ask: does this make the document more thorough and accurate, or less? If less: reject it unconditionally.**

### Rule 3 — Review agents must audit for integrity violations

Every review agent invocation (steps 1b, 2a, 2b, 3a, 6, 7, 8) must include this standing instruction in addition to its specific task:

> "Before reporting issues: check whether any previous agent has (a) cited something as out of scope without a matching entry in the design document's Out of scope section, or (b) edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as a critical issue — highest priority, before any other finding."

---

## Context

TIMESTAMP_LTZ is already fully implemented and serves as the exact template:
- Gherkin definition: `tests/definitions/shared/types/timestamp_ltz.feature`
- Python tests: `python/tests/e2e/types/test_timestamp_ltz.py`
- Shared test utilities: `python/tests/e2e/types/utils.py`
- Design doc pattern: `doc/ai/feature-descriptions/` (use LTZ's structure as the model)

**Key difference — NTZ vs LTZ:**

| | TIMESTAMP_NTZ | TIMESTAMP_LTZ |
|---|---|---|
| Timezone in Python | `None` (naive datetime) | set (aware datetime) |
| `assert_datetime_type` | `require_tzinfo=False` | `require_tzinfo=True` |
| SQL literal | `'2024-01-15 10:30:00'::TIMESTAMP_NTZ` | `'2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ` |
| UTC normalization | not needed — direct `==` | via `to_utc()` |
| Session timezone effect | none | values shift per session TZ |

---

## Feedback Loop Protocol

Several steps below end with a **review loop**. Every review loop follows this exact pattern:

```
REPEAT:
  1. Launch reviewer agent (specified per step)
  2. Instruct it: "Find the top 3 issues by importance. Stop at 3 — do not list more."
  3. Fix all 3 issues
  4. Re-launch the same reviewer agent
UNTIL: reviewer reports 0 critical/high issues
```

The "top 3 only" constraint keeps each iteration focused. If the reviewer finds fewer than 3 issues, fix them all and stop — do not keep looping once no critical issues remain.

---

## Commit Strategy

Follow a **top-down, granular commit discipline** throughout the task.

**Commit after every meaningful, stable state.** Do not accumulate work across multiple phases into a single commit. Each commit should represent one logical step forward.

**Top-down ordering:** commits must flow in dependency order — the design document before the Gherkin, the Gherkin before the Python tests, the tests before any review fixes. Never commit a later artefact without the earlier ones already committed.

**Every commit must leave a non-failing environment.** Before committing:
- No previously-passing tests may be newly broken
- No syntax errors in any committed file
- The validator (`./tests/tests_format_validator/run_validator.sh`) must exit cleanly if a `.feature` file is included

**Each commit must be granular and self-contained:** one concern per commit. Examples of correct granularity:
- `doc: write TIMESTAMP_NTZ design document first draft`
- `doc: refine design document after clarity loop — binding behaviour clarified`
- `test: add timestamp_ntz Gherkin feature definition`
- `test: fix gherkin-expert issues — rename scenarios to start with 'should'`
- `test: implement Python e2e tests for timestamp_ntz`
- `test: fix test-quality-reviewer issues — add value assertions to large result set test`

**Commit message must explain what changed and why** — not just "update file." A reader must be able to understand the commit without reading the diff.

Mandatory commit points are marked **[COMMIT]** in the steps below. Do not skip them; do not merge them.

---

## Steps

### 0. Setup environment

**Agents: 1 × dev-env-fixes — only if environment is broken (on demand)**

From the `python/` directory, verify you can run both test modes:

```bash
# Universal driver (dev mode — editable install, reflects source changes)
hatch run dev:e2e -k timestamp_ltz

# Reference driver (official snowflake-connector-python)
PYTHON_REFERENCE_DRIVER_VERSION=3.17.2 hatch run reference:test -k timestamp_ltz
```

Both should find and pass the existing LTZ tests. If either command fails due to environment issues (missing libraries, build errors, GLIBCXX conflicts, hatch not found, etc.), use the dev-env-fixes agent (`/home/fpawlowski/.claude/agents/temp/dev-env-fixes.md`) to diagnose and fix before proceeding.

---

### 1. Research — run all four agents in parallel

**Agents: 4 × in parallel — Agents A and B are snowflake-driver-expert; Agents C and D are general-purpose**

Launch these four subagents simultaneously. As each result arrives, write its findings into `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` — the document grows progressively throughout this step. Do not wait until all four finish before writing; update the doc after each agent returns.

**Agent A — snowflake-driver-expert: universal driver NTZ source**
Investigate how TIMESTAMP_NTZ is handled in the Python wrapper source:
- `python/src/snowflake/connector/_internal/binding_converters.py` — what SQL type does a naive datetime (no tzinfo) bind as? What about an aware datetime bound to NTZ?
- `python/src/snowflake/connector/_internal/type_codes.py` — NTZ type code
- `python/src/snowflake/connector/_internal/nanoarrow_cpp/ArrowIterator/` — is there a separate NTZ converter, or does the same converter handle both NTZ and LTZ? Does it set tzinfo?
- Search `python/src/` for "TIMESTAMP_NTZ" and "NTZ"

**Agent B — snowflake-driver-expert: old driver behaviour and LTZ coverage mapping**
Two goals in one agent:
1. Search `/home/fpawlowski/repo/snowflake-connector-python/` for any existing TIMESTAMP_NTZ tests and handling. How does the old driver return NTZ? Does it return a naive datetime (tzinfo=None)? Find file:line evidence.
2. Read `tests/definitions/shared/types/timestamp_ltz.feature` and `python/tests/e2e/types/test_timestamp_ltz.py` and produce a complete mapping of every tested scenario to its NTZ equivalent. For each, state: applies unchanged / needs modification / needs new NTZ-specific variant. Pay attention to the binding test class docstring — session-TZ coupling differs for NTZ.

Every scenario we write must match old driver behaviour. Any deviation must be explicitly justified.

**Agent C — general-purpose: Snowflake docs on TIMESTAMP_NTZ**
Fetch https://docs.snowflake.com/en/sql-reference/data-types-datetime and summarise for TIMESTAMP_NTZ:
- Storage format (is it wall-clock or UTC internally?)
- Default and max precision (microseconds? nanoseconds?)
- Synonyms: DATETIME, TIMESTAMP — are they aliases for NTZ?
- Min/max representable values worth testing
- Whether session TIMEZONE affects returned values (it should not)
- What Python type the official connector returns (naive datetime?)

**Agent D — general-purpose: Snowflake docs on binding and edge cases**
Fetch https://docs.snowflake.com/en/developer-guide/python-connector/python-connector-api and https://docs.snowflake.com/en/sql-reference/data-types-datetime#special-values and answer:
- What SQL type does the Python connector bind a `datetime` object as by default?
- What happens when a timezone-aware datetime is bound to `?::TIMESTAMP_NTZ` — does it strip tzinfo, error, or convert?
- Any special timestamp values (max date, min date) worth including as edge-case test scenarios?

#### 1a. Write the design document

Once all four agents have returned, write a complete first draft of `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md`. The document must cover:

- **What TIMESTAMP_NTZ is** — Snowflake semantics, storage format, precision, synonyms
- **Python return type** — naive datetime, tzinfo=None; with file:line evidence from the driver source and old driver
- **Binding behaviour** — what happens when naive vs aware datetimes are bound to NTZ; exact SQL type used
- **Session timezone independence** — confirmed with evidence
- **Differences from TIMESTAMP_LTZ** — explicit side-by-side comparison
- **Scenario plan** — the full list of test scenarios we will write, with rationale for each; for each scenario state the old driver reference (file:line) or justify why it is new
- **Edge cases** — any values or conditions beyond the standard LTZ set, with justification
- **Out of scope** — explicit list of what we are not testing and why

This document is the implementation contract. Step 2 (Gherkin) is derived from it, not the other way around.

#### 1b. Design document clarity loop

**Agents per iteration: 1 × general-purpose (reviewer) + up to 3 × subagents in parallel (1 per question — snowflake-driver-expert or general-purpose depending on question type)**

Before proceeding to step 2, the design document must be free of vague terms, ambiguous claims, and unanswered questions. Apply the following loop:

```
REPEAT:
  1. Launch 1 general-purpose agent and instruct it:
     "Read doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md.

      Integrity check first: has any scenario been removed from the
      scenario plan since the document was first written? Has any
      requirement been weakened or made vaguer? Has any new 'Out of
      scope' entry appeared that was not in the original draft? If so,
      report it as a critical issue before anything else — it must be
      reverted, not accepted.

      Then: find every claim that is vague, ambiguous, unverified, or
      relies on an assumption not confirmed with evidence. List each as
      a specific question (e.g. 'Does binding a tz-aware datetime to
      NTZ silently strip tzinfo or raise an error?').
      Stop at 3 questions — the most important ones only."

  2. For each question (up to 3), launch one subagent in parallel:
     - Driver source questions → 1 × snowflake-driver-expert
     - Snowflake behaviour / docs questions → 1 × general-purpose (WebFetch)
     All questions may be sent to their respective agents simultaneously.

  3. Update the design document with the concrete answer,
     replacing the vague claim with a specific, evidenced statement.
     Never remove or weaken anything already written.

UNTIL: the general-purpose agent finds no unclear points and no integrity violations
```

**[COMMIT]** `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` once the loop exits. Message: `doc: write TIMESTAMP_NTZ design document — clarity loop clean`

---

### 2. Create Gherkin definition

**Agents: 1 × snowflake-universal-driver**

**Input**: the completed design document from step 1.

Create `tests/definitions/shared/types/timestamp_ntz.feature` derived directly from the scenario plan in the design document. Every scenario in the feature file must be traceable to a row in the design document's scenario plan. If a scenario cannot be traced, either add it to the design document first or remove it from the feature file.

Mandatory style rules (from `.ai/review/universal-driver-tests.yaml`):
- Scenario names must start with lowercase "should"
- Use `Scenario Outline` + `Examples` for all parametrised cases
- One `When` per scenario
- No repeated When/Then blocks for different inputs — use Examples table instead

Changes from LTZ:
- Keep `@python` feature tag
- SQL literals: drop `+00:00` offset — `'2024-01-15 10:30:00'::TIMESTAMP_NTZ`
- Assertions reflect naive datetime semantics (no timezone info)
- Scenario names and section structure must mirror LTZ for consistency

After creating the feature file, update the design document to reference the feature file and mark the scenario plan as "implemented in Gherkin."

**[COMMIT]** `tests/definitions/shared/types/timestamp_ntz.feature` + `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` together. Message: `test: add timestamp_ntz Gherkin feature definition`

#### 2a. Gherkin quality loop

**Agents per iteration: 1 × gherkin-expert**

Apply the **Feedback Loop Protocol** using `/home/fpawlowski/.claude/agents/temp/gherkin-expert.md`.

Instruct the agent on each iteration:
> "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue before anything else.
> Then: review `tests/definitions/shared/types/timestamp_ntz.feature` following the 5-step process from your instructions. Find the top 3 issues by importance — completeness gaps, vague steps, missing Examples coverage, wrong tags, or anti-patterns. Stop at 3. Do not list more."

After each fix, update the design document if the fix changes the scenario plan (e.g. a scenario was added or split).

**[COMMIT]** after each iteration that changes files. Message: `test: fix gherkin-expert issues — <one-line summary of what changed>`

#### 2b. Gherkin validator loop

**Agents per iteration: 1 × gherkin-validator-expert**

Apply the **Feedback Loop Protocol** using `/home/fpawlowski/.claude/agents/temp/gherkin-validator-expert.md`.

Instruct the agent on each iteration:
> "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue before anything else.
> Then: run `./tests/tests_format_validator/run_validator.sh` and report the top 3 issues it surfaces for `timestamp_ntz.feature` — orphaned scenarios, name mismatches, missing tags, or format errors. Stop at 3."

Fix the 3 issues, re-run the validator, and repeat until it exits with 0 errors and 0 warnings.

**[COMMIT]** after each iteration that changes files. Message: `test: fix gherkin-validator issues — <one-line summary>`

---

### 3. Implement Python tests

**Agents: 1 × snowflake-universal-driver**

**Input**: the completed Gherkin feature file from step 2. Every test method must correspond to a scenario in the feature file. Do not add tests that have no Gherkin scenario.

Create `python/tests/e2e/types/test_timestamp_ntz.py` modelled on `test_timestamp_ltz.py`.

Required changes from LTZ:
- All `assert_datetime_type` calls: `require_tzinfo=False`
- Expected values: naive datetimes — `datetime(2024, 1, 15, 10, 30, 0)` (no `tzinfo`)
- No `to_utc()` helper; compare values directly with `==`
- No `compare_ts_utc` comparator — use default equality in `assert_sequential_values`
- Binding test class docstring: copy the exact wording from the design document's "Binding behaviour" section

**[COMMIT]** `python/tests/e2e/types/test_timestamp_ntz.py` once the initial file is written (before quality loop). Message: `test: implement Python e2e tests for timestamp_ntz`

#### 3a. Test quality loop

**Agents per iteration: 1 × test-quality-reviewer**

Apply the **Feedback Loop Protocol** using `/home/fpawlowski/.claude/agents/temp/test-quality-reviewer.md`.

Instruct the agent on each iteration:
> "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue before anything else.
> Then: review `python/tests/e2e/types/test_timestamp_ntz.py`. Find the top 3 issues by importance — fake tests, missing assertions, wrong assertion values, weak assertions (e.g. only checking type not value), tautologies, or tests that would pass even if the driver were broken. Stop at 3. Do not list more."

Fix the 3 issues, then repeat until the agent finds no critical or high-priority issues.

**[COMMIT]** after each iteration that changes files. Message: `test: fix test-quality-reviewer issues — <one-line summary>`

---

### 4. Run reference driver tests (criterion a) ← PRIMARY GATE

**Agents: 0 — shell commands only**

```bash
cd python
PYTHON_REFERENCE_DRIVER_VERSION=3.17.2 hatch run reference:test -k timestamp_ntz
```

**All new tests must pass on the reference driver before anything else.** The reference driver defines correct behaviour. If a test fails here, the test is wrong — fix the test, not the driver. Do not proceed to step 5 until this is green.

---

### 5. Run universal driver tests (criterion b)

**Agents: 0 — shell commands only**

```bash
cd python
hatch run dev:e2e -k timestamp_ntz
```

If you changed Rust core, rebuild first:
```bash
cargo build -p sf_core   # from repo root
```

All new tests must pass.

---

### 6. Implementation honesty loop

**Agents per iteration: 1 × gherkin-implementation-reviewer**

Once both test suites pass, apply the **Feedback Loop Protocol** using `/home/fpawlowski/.claude/agents/temp/gherkin-implementation-reviewer.md`.

Instruct the agent on each iteration:
> "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue before anything else.
> Then: audit `tests/definitions/shared/types/timestamp_ntz.feature` against `python/tests/e2e/types/test_timestamp_ntz.py`. Find the top 3 issues by importance — Gherkin Given/When/Then claims that don't match the implementation, configuration set but never used, assertions that are too weak to catch real failures, or architecture violations. Stop at 3."

Fix the 3 issues (which may require editing either the feature file or the test file). If the feature file changes, also update the design document to keep it consistent. Repeat until the agent finds no critical or high-priority issues.

**[COMMIT]** after each iteration that changes files. Message: `test: fix gherkin-implementation-reviewer issues — <one-line summary>`

---

### 7. Final review

**Agents: 2 × snowflake-universal-driver, in parallel (Agents E and F); re-run each once after fixes**

Launch these two agents simultaneously. Apply the **Feedback Loop Protocol** for each independently. Address all findings before declaring done.

**Agent E — snowflake-universal-driver: implementation review**
Review `tests/definitions/shared/types/timestamp_ntz.feature` and `python/tests/e2e/types/test_timestamp_ntz.py` against:
- Naming consistency with the other type tests (ltz, boolean, decfloat, float, int, number)
- Gherkin rules from `.ai/review/universal-driver-tests.yaml` — check every rule
- DRY: no repeated logic that could use existing utilities from `utils.py`
- No unnecessary code, no commented-out blocks, no unused imports

Instruct: "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue. Then: find the top 3 issues by importance. Stop at 3."

**Agent F — snowflake-universal-driver: adversarial review**
Independently read the two new files and argue against them:
- Are there any scenarios that are wrong (assert the wrong thing)?
- Are there scenarios that are missing (gaps relative to what LTZ tests)?
- Is the binding test docstring accurate given actual driver behaviour?
- Would any of these tests produce a false positive (pass even if the driver is broken)?

Instruct: "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited the design document to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as the top critical issue. Then: find the top 3 issues by importance. Stop at 3."

Fix all issues from both agents, then re-run each agent once more to confirm no critical issues remain.

**[COMMIT]** after fixing findings from each agent. Message: `test: fix final-review issues — <one-line summary>`

---

### 8. Clean code review loop (adversarial two-reviewer process)

**Agents per iteration: 2 × clean-code-reviewer, sequential — Reviewer A runs first, then Reviewer B reviews A's output**

Run this after all previous loops are clean — this is the final gate before the task is done.

Each iteration of this loop uses **two independent clean-code-reviewer agents running sequentially**:

#### Iteration structure

**Reviewer A** — proposes improvements
Launch `/home/fpawlowski/.claude/agents/temp/clean-code-reviewer.md` and instruct:
> "First: check whether any agent has cited something as out of scope without a matching entry in the design document's Out of scope section, or has edited `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` to remove, weaken, or vague-ify any requirement or scenario. If you find either, report it as a critical issue before any code review findings — it must be reverted.
> Then: review `python/tests/e2e/types/test_timestamp_ntz.py` and `tests/definitions/shared/types/timestamp_ntz.feature`. List every improvement you can find — naming inconsistencies with sibling type tests, DRY violations, magic values that should be named constants, misleading comments, unnecessary complexity, or anything that would not pass code review. Be specific: quote the exact code and explain what the improvement is."

**Reviewer B** — challenges each proposed improvement
Launch a second, independent instance of `/home/fpawlowski/.claude/agents/temp/clean-code-reviewer.md` and pass it the full list from Reviewer A. Instruct:
> "You are given a list of proposed clean code improvements. For each one, give a verdict: ACCEPT (genuine improvement, clearly makes the code better) or REJECT (forcefully invented — the change would not meaningfully improve readability, correctness, or maintainability, or it introduces new problems). Be adversarial: reject anything that is nitpicking for its own sake, that trades one style for another equivalent style, or that adds indirection without real benefit. Justify each verdict in one sentence."

**Decision rule:**
- Only implement improvements that Reviewer B verdicts as ACCEPT.
- Discard all REJECTs without implementing them.

**Loop exit condition:**
- If Reviewer A finds no improvements, the loop ends.
- If Reviewer A finds improvements but Reviewer B rejects all of them, the loop also ends — no real improvements remain.
- If at least one ACCEPT exists, implement those, then start the next iteration from Reviewer A again.

**[COMMIT]** after each iteration where ACCEPTs were implemented. Message: `refactor: clean-code-reviewer improvements — <one-line summary of accepted changes>`

---

## Completion Checklist

The task is complete only when all of the following are true:

- [ ] `doc/ai/feature-descriptions/datatypes/timestamp/timestamp-ntz-design.md` is complete, committed, and has no vague claims (step 1b loop clean)
- [ ] No scenario has been removed from the design doc's scenario plan since the first draft
- [ ] Every "Out of scope" entry in the design doc has an explicit justification (not added retroactively to avoid work)
- [ ] `tests/definitions/shared/types/timestamp_ntz.feature` is committed and every scenario traces to the design document
- [ ] Reference driver tests pass (step 4)
- [ ] Universal driver tests pass (step 5)
- [ ] gherkin-expert loop: no critical issues
- [ ] gherkin-validator-expert: 0 errors, 0 warnings
- [ ] test-quality-reviewer loop: no critical issues
- [ ] gherkin-implementation-reviewer loop: no critical issues
- [ ] clean-code-reviewer adversarial loop: Reviewer A finds nothing, or Reviewer B rejects everything Reviewer A finds
