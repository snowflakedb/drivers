---
description: "Drivers knowledge base — the Universal Driver: the Rust core `sf_core`, the protobuf/FFI boundary and its bridge crates, and the JDBC, ODBC, Python, Node.js and .NET wrappers. Use when: building or running UD tests for a language wrapper, mapping old-driver test coverage onto the UD, adding a test or a Gherkin scenario, reviewing driver test code, reading driver CI results across GitHub Actions, Buildkite and Jenkins, or querying driver test and coverage data in Snowhouse."
argument-hint: "[section: concepts | architecture | tools | operations]"
---

# Drivers Knowledge Base

The single index over this repo's reference knowledge, organized into **sections**. Load a section index for its catalog, then load the specific reference. Totoroid registers this repo under the area `sdet-non-monorepo`.

This repo is the **Universal Driver** (UD): one Rust core, `sf_core`, reached across a protobuf and FFI boundary by thin per-language wrappers — `jdbc` over `jdbc_bridge`, `nodejs` over `nodejs_bridge`, `python` over `python_bridge`, plus `odbc`, which links the core directly, and `dotnet`. The wrappers are at different stages, and `gosnowflake/` is still a placeholder rather than a driver; the status of each is in [references/architecture/components/drivers.md](references/architecture/components/drivers.md). It replaces the legacy standalone drivers, and `tests/oldTestsCoverage/{jdbc,odbc,python}.yaml` records which of their tests the UD now covers. Each wrapper carries its own toolchain — cargo, Gradle, cmake, hatch, npm — so a build or test command is per-wrapper rather than repo-wide, and CI spans GitHub Actions, `.buildkite/pipelines`, and the `ci/Jenkinsfile.*` jobs.

## Scope

| Repo | Paths owned | Source model |
|---|---|---|
| drivers | `**` | `.ai` for all reference knowledge; `.claude` for the `alwaysApply` rules and the `sf ai skills` lifecycle skills, which stay engine-native by design |

## Sections

| Section | Address | Index | Covers |
|---|---|---|---|
| concepts | `/kb concepts` | [references/concepts/README.md](references/concepts/README.md) | Cross-cutting behavior spanning several wrappers: the core-and-bridge split across the protobuf and FFI boundary, declared behavior differences from the legacy drivers, connection parameters generated from one spec, and how legacy-test coverage is attributed to the UD. |
| architecture | `/kb architecture` | [references/architecture/README.md](references/architecture/README.md) | What the repo ships and what each shipped driver's status is, `sf_core` and the bridge and supporting crates behind them, the legacy driver repos the UD replaces, and how to develop and test this code. |
| tools | `/kb tools` | [references/tools/README.md](references/tools/README.md) | How an agent drives a thing and reads its data: the CI surfaces, Snowhouse test and coverage tables, Wiremock, credentials, and uSUT. |
| operations | `/kb operations` | [references/operations/README.md](references/operations/README.md) | Live-system duties needing human approval, and the recurring driver reports. |

## Section-address convention

- `/kb` — this index; pick a section.
- `/kb <section>` — a section index, then pick a leaf.
- `/kb <section> <leaf>` — load a specific reference directly.

The slug is unprefixed; this repo hosts one area. Cross-references within the kb use paths relative to this file as `references/<section>/<subpath>`, never slash-command syntax.

Each section's catalog is its `README.md`, which carries no frontmatter — `sf ai rules build` skips any file named `readme.md`. Every other file under `references/` sets `no-pointer: true` so the build leaves it as reference content instead of generating a slash command for it.

Authoring rules live in [CONTRIBUTING.md](CONTRIBUTING.md) and in each section's own `CONTRIBUTING.md`: the catalogs are the read path, those files are the update path.

## Old skill slugs redirect here

Ten skills that previously held this knowledge now carry a one-line redirect at `.claude/skills/<name>/SKILL.md`, so an existing invocation still resolves. The content is in this kb.

| Old slug | Now |
|---|---|
| `run-jdbc-ud-tests` | [references/tools/jdbc-ud-tests.md](references/tools/jdbc-ud-tests.md) |
| `run-odbc-ud-tests` | [references/tools/odbc-ud-tests.md](references/tools/odbc-ud-tests.md) |
| `run-nodejs-ud-tests` | [references/tools/nodejs-ud-tests.md](references/tools/nodejs-ud-tests.md) |
| `run-python-ud-tests` | [references/tools/python-ud-tests.md](references/tools/python-ud-tests.md) |
| `test-coverage-mapper` | [references/tools/old-driver-coverage-mapping.md](references/tools/old-driver-coverage-mapping.md) |
| `bug-regression-coverage` | [references/tools/bug-regression-coverage.md](references/tools/bug-regression-coverage.md) |
| `add-tests` | [references/architecture/best-practices/adding-tests.md](references/architecture/best-practices/adding-tests.md) |
| `jdbc-test-reviewer` | [references/architecture/best-practices/jdbc-test-review.md](references/architecture/best-practices/jdbc-test-review.md) |
| `format` | [references/architecture/best-practices/formatting.md](references/architecture/best-practices/formatting.md) |
| `end-of-task-self-review` | [references/architecture/best-practices/end-of-task-review.md](references/architecture/best-practices/end-of-task-review.md) |

`/odbc-weekly-report` redirects the same way, to [references/operations/odbc-weekly-report.md](references/operations/odbc-weekly-report.md).

The eleven on-demand Cursor rules and the two `@`-included `.claude/rules/` reference docs also moved in; their content is now under `references/`.

## What deliberately stays engine-native

| Stays | Why |
|---|---|
| The five `alwaysApply: true` rules in `.claude/rules/` | They are injected into the system prompt at session start, which a kb reference loaded on demand does not replicate. |
| The six `sf ai skills` lifecycle skills | Repo tooling installed by `sf ai skills repo-setup`, identical across repos, not area knowledge. |
| `.cursor/commands/migrate-nodejs-test.md` | An invocable step-by-step workflow. Folding it into a reference page would remove what its callers invoke. |
| `.cursor/commands/review.md` | Orchestration over the `.ai/review/` rulesets. |
| `.ai/review/*.yaml`, `.ai/feature_prompts/` | A separate review mechanism and per-feature prompts; both remain discovery sources for this kb. |
