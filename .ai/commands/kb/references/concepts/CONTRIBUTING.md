---
description: Authoring rules for drivers concepts references — file structure, registration, and discovery.
no-pointer: true
---

# Contributing to Concepts References

Follow the shared rules in [../../CONTRIBUTING.md](../../CONTRIBUTING.md), plus this section's own rules. Every rule here obeys [../../CONTRIBUTING.md § The laws govern this file too](../../CONTRIBUTING.md#the-laws-govern-this-file-too).

## File structure

A concept file opens with one paragraph stating the behavior and which wrappers or crates take part in it, then uses the headings that carry weight for that concept:

- **How it works** — the mechanism, following the call or the data from one named unit to the next.
- **Where it lives** — the repo-relative paths and symbols that implement it, so a reader can go read the code.
- **What varies per wrapper** — a table when the behavior differs across `jdbc`, `odbc`, `python`, `nodejs`, or `dotnet`. State which wrappers a claim actually covers rather than writing "every wrapper"; they are at different stages, and `gosnowflake/` is a placeholder. Omit the heading when the behavior does not vary.
- **Constraints** — what a change here must preserve, such as an FFI or protobuf contract, or a behavior a legacy driver established.

## Registration

- Place the file at `kb/references/concepts/<concept>.md`.
- Add a row to the catalog in [`README.md`](README.md), keeping the table's sort order.
- Run `sf ai rules build`, then `sf ai rules lint`.

## Discovery prompts

- `adr/` — the accepted design decisions, and the constraint each one fixes.
- `protobuf/`, `proto_generator/`, `proto_utils/` — the wire contract between core and wrappers.
- `error_trace/`, `error_trace_derive/` — how an error crosses the boundary and reaches a caller.
- `sf_params_spec/`, `sf_params_codegen/` — how a connection parameter is defined once, and which wrappers actually get a generated view of it (check the pre-commit hooks; today only Python does).
- `scripts/validate_behavior_differences.py` and the behavior-difference data it checks — where the UD is allowed to diverge from a legacy driver.
