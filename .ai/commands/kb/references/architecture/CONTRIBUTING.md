---
description: Authoring rules for drivers architecture references — subsection choice, file structure, registration, and discovery.
no-pointer: true
---

# Contributing to Architecture References

Follow the shared rules in [../../CONTRIBUTING.md](../../CONTRIBUTING.md), plus this section's own rules. Every rule here obeys [../../CONTRIBUTING.md § The laws govern this file too](../../CONTRIBUTING.md#the-laws-govern-this-file-too).

## Which subsection

| Subsection | Holds |
|---|---|
| `components/` | A driver this repo ships or intends to ship to a user — `jdbc`, `odbc`, `python`, `nodejs`, `dotnet`, and the `gosnowflake` placeholder — plus standalone binaries such as `odbc_trace_tool`. Record each one's release status; they are not all shipped. |
| `libraries/` | A crate the components import and do not run on their own: `sf_core`, the `*_bridge` crates, `error_trace`, `proto_utils`, `sf_params_spec`, `sf_mini_core`. |
| `repos/` | A repo this area reads or contributes to without owning it, including the legacy driver repos the UD replaces. |
| `best-practices/` | How to develop the code: toolchain setup, language conventions, test layout, review rulesets. |

Create a subsection directory when its first file lands. Give a subsection its own `README.md` only when its row count outgrows the grouped catalog in [`README.md`](README.md); until then that parent catalogs everything.

A driver that both ships to users and has an agent-facing command surface is placed here for how it is built, and in [../tools/](../tools/) for how an agent drives it, with a link each way.

## File structure

- **What it is** — one paragraph: the unit's job and its place in the core-plus-bridge split.
- **Layout** — the repo-relative directories and the entry-point symbols.
- **Build and test** — the toolchain and the working directory, naming the skill that holds the runnable steps.
- **Interfaces** — what it exposes to the units above it and depends on below.
- **Gotchas** — the constraints that are not visible from the code alone.

For a `repos/` file, replace **Build and test** and **Interfaces** with what this area uses that repo for and what it must not change there.

## Registration

- Place the file at `kb/references/architecture/<subsection>/<name>.md`.
- Add a row under that subsection's heading in [`README.md`](README.md), keeping the table's sort order.
- Run `sf ai rules build`, then `sf ai rules lint`.

## Discovery prompts

- `Cargo.toml` `[workspace] members` — the Rust crates, each a `libraries/` or `components/` candidate.
- The per-wrapper directories `jdbc/`, `odbc/`, `nodejs/`, `python/`, `dotnet/`, `gosnowflake/` and their build files.
- `repos/legacy-drivers.md` already catalogues the legacy driver repos; extend it rather than adding a second file.
- `.cursor/rules/*.mdc` — the Rust, ODBC, and test-generation conventions, for `best-practices/`.
- `.ai/review/*.yaml` — the review rulesets, each naming a standard this code is held to.
- `adr/`, `docs/`, `CONTRIBUTING.md` at the repo root.
