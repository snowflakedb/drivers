---
description: Authoring rules for the drivers knowledge base, and a pointer to each section's CONTRIBUTING.
no-pointer: true
---

# Contributing to the Drivers Knowledge Base

Read the relevant section's CONTRIBUTING before editing — each section owns one kind of knowledge and has its own conventions. These extend the global laws in `/update-kb`; they do not restate them.

## The laws govern this file too

`/update-kb`'s laws apply to the rules written here, not only to kb entries:

- **Instruct, do not argue** — state the rule with no "because" clause defending a boundary, and no forward-looking "will change once …" framing.
- **Link, do not copy** — never reproduce a law **or any of its sub-bullets**. An area rule narrows, specializes, or adds; it does not copy. Link a neighbouring section's CONTRIBUTING rather than restating its boundary from the other side.

## Conventions for every section

- **Name a wrapper by its directory.** `jdbc`, `odbc`, `python`, `nodejs`, `dotnet`, `gosnowflake` — and its Rust bridge crate by the crate name, `jdbc_bridge`, `nodejs_bridge`, `python_bridge`. Write "the Python wrapper", not "the Python connector", which names the legacy driver.
- **Cite a Rust symbol as `crate::path::Symbol`** and a path as repo-relative from the root: `sf_core/src/…`, `tests/oldTestsCoverage/python.yaml`.
- **Give a command its working directory** when it is not the repo root, since each wrapper builds with its own toolchain.
- **Distinguish the UD from the driver it replaces.** Where both exist, say which one a statement is about; `architecture/repos/` holds the legacy repos.
- **Reference an existing skill rather than duplicating its procedure.** A kb file explains what a thing is and why a step matters; the runnable steps stay in the skill it names.

## Section conventions

| Section | CONTRIBUTING |
|---|---|
| `concepts/` | [references/concepts/CONTRIBUTING.md](references/concepts/CONTRIBUTING.md) |
| `architecture/` | [references/architecture/CONTRIBUTING.md](references/architecture/CONTRIBUTING.md) |
| `tools/` | [references/tools/CONTRIBUTING.md](references/tools/CONTRIBUTING.md) |
| `operations/` | [references/operations/CONTRIBUTING.md](references/operations/CONTRIBUTING.md) |

## Build and lint

`.ai/commands/kb/` is the hand-authored source. After editing, run `sf ai rules build`, then `sf ai rules lint`. Never hand-edit the generated `.claude/commands/kb/` or `.cursor/rules/kb/` files.

The repo's other agent files — `.claude/rules/*.md` mirrored into `.cursor/rules/*.mdc`, and `.claude/skills/*/SKILL.md` mirrored into `.cursor/skills/` — are hand-authored on the Claude side and checked by the `ai-rules-sync` pre-commit hook. `sf ai rules build` neither reads nor writes them.
