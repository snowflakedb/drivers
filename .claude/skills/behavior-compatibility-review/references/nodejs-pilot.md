# Node.js pilot

The first rollout is manual and slash-command only. The pilot measures finding
quality before this skill is connected to `end-of-task-self-review` or made
model-invocable.

## Who and when

Ask Node.js developers to run `/behavior-compatibility-review` on active feature
or test branches before pushing or opening a PR. Prefer changes that touch
observable behavior: connection options, authentication, statements, results,
errors, data types, or wire payloads.

Do not use coverage-mapping-only or documentation-only branches as the whole
sample. Those are useful negative controls but do not exercise finding quality.

## Run

1. Finish the intended implementation and tests locally.
2. Run `/behavior-compatibility-review` without first telling the agent what
   finding is expected.
3. Read each finding before changing the branch.
4. Record whether it is actionable, a false positive, already known/tracked, or
   unsupported by enough evidence.
5. Complete normal human review and record any compatibility issue the skill
   missed.

The command is read-only. Developers decide whether to fix code, add tests, or
document a behavior difference in a separate step.

## Feedback template

```text
Driver / feature:
Diff size and main surfaces:
Useful findings:
False positives:
Missed compatibility or test gaps found later:
Was the evidence sufficient to verify each finding?
Did the review stay scoped to the diff?
Approximate review time:
Suggested instruction or output change:
```

Post feedback as a comment on the skill PR, including after it merges, so the
initial sample has one durable collection point. Link concrete examples rather
than copying customer data or credentials.

## Exit criteria

Review the pilot after at least five real Node.js changes from at least two
developers.

The skill is ready for a follow-up that considers model invocation or an
`end-of-task-self-review` hook when:

- it produces no suite-wide backlog dumps;
- reviewers can verify every reported behavior claim from the cited evidence;
- at least four out of five non-`insufficient_evidence` findings are judged
  actionable, and existing BDs are correctly placed in `Already tracked`;
- known compatibility issues found during ordinary review are added as missed
  cases and addressed in the skill before enabling automatic invocation.

If the sample does not meet these gates, revise the workflow and repeat the
manual pilot. Do not compensate by weakening the relevance gate.
