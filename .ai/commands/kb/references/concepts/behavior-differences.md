---
description: How an intentional divergence between the Universal Driver and the legacy driver is declared and enforced — the per-wrapper BehaviorDifferences.yaml files, their schema and enums, and the pre-commit validator that gates them.
no-pointer: true
---

# Behavior Differences from the Legacy Drivers

The Universal Driver is allowed to behave differently from the driver it replaces, but never silently. Each divergence is a numbered entry in the wrapper's `BehaviorDifferences.yaml`, and a pre-commit validator rejects a malformed or duplicated one.

## Where they live

Four files, one per wrapper that has one, at paths the validator hard-codes. `.NET` and Go have none — do not create one without adding it to the validator's list, or it will never be checked.

| Wrapper | Path |
|---|---|
| ODBC | `odbc_tests/BehaviorDifferences.yaml` |
| Python | `python/BehaviorDifferences.yaml` |
| JDBC | `jdbc/BehaviorDifferences.yaml` |
| Node.js | `nodejs/BehaviorDifferences.yaml` |

There is no core-level file: a divergence is a property of how a wrapper presents behavior, so it is recorded against that wrapper.

## Entry schema

Every entry sits under a top-level `behavior_differences:` map, keyed by a unique positive integer.

Required on each entry:

| Field | Meaning |
|---|---|
| `name` | Short non-empty title |
| `status` | `unknown`, `todo`, `fixed`, or `allowed` |
| `type` | `unknown`, `api_incompatibility`, `bug`, `bugfix`, or `enhancement` |
| `impact` | `low`, `medium`, or `high` |
| `is_breaking_change` | `true` when existing application code may fail or return different results after switching drivers with no code change; `false` for a pure improvement old callers get for free |

Optional: `reviewed` (triage flag), `status_rationale`, `description`, `old_driver_behavior`, `new_driver_behavior`.

A new entry conventionally starts at `impact: high` and `is_breaking_change: true`. The validator does not fill these in, so an entry that is genuinely lower-impact must say so deliberately.

## What the validator enforces

`scripts/validate_behavior_differences.py`, wired as the `behavior-differences-validator` pre-commit hook:

- IDs are unique positive integers. Uniqueness is checked against the **raw file text**, not the parsed mapping, since `yaml.safe_load` keeps only the last of a duplicated key and would otherwise hide the collision. The raw ID-line count must equal the number of unique parsed entries.
- ODBC only: when `reviewed` is `true`, `status_rationale` must be a non-empty string.
- It reads the committed version of each file with `git show HEAD:<path>`, so it compares your change against committed state rather than the working tree.

## Constraints

- Adding an entry by copying a neighbouring one is the common way to produce a duplicate ID. The raw-text check catches it; a YAML-aware editor would not.
- A behavior difference is not test coverage. Whether a legacy test's scenario is covered at all is tracked separately — see [old-driver-coverage-attribution.md](old-driver-coverage-attribution.md).
- The review ruleset `.ai/review/universal-driver-behavior-differences.yaml` also inspects these entries during AI code review.
