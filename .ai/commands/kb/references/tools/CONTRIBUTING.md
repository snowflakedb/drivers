---
description: Authoring rules for drivers tools references — file structure, registration, and discovery.
no-pointer: true
---

# Contributing to Tools References

Follow the shared rules in [../../CONTRIBUTING.md](../../CONTRIBUTING.md), plus this section's own rules. Every rule here obeys [../../CONTRIBUTING.md § The laws govern this file too](../../CONTRIBUTING.md#the-laws-govern-this-file-too).

## File structure

- **What it gives you** — the question this tool answers, stated so an agent can tell whether to reach for it.
- **Access** — how to reach it: the account, role, or credential, and where that comes from. Name the credential; never write its value.
- **Usage** — the invocation or query, with the fields an agent actually selects on. A runnable query goes in a fenced block.
- **Interpreting the result** — what a value means, and the values that mean something is wrong.
- **Limits** — retention, sampling, lag, and rate limits, so a reader does not draw a conclusion the data cannot support.

A data object gets a table of the columns worth selecting rather than a full schema dump.

## Registration

- Place the file at `kb/references/tools/<tool>.md`.
- Add a row to the catalog in [`README.md`](README.md), keeping the table's sort order.
- Run `sf ai rules build`, then `sf ai rules lint`.

## Discovery prompts

- `.github/workflows/` — the GitHub Actions surface, including the `mirror*` and `test-*` workflows.
- `.buildkite/pipelines/` and `ci/Jenkinsfile.*` — the Buildkite and Jenkins jobs, including coverage, uSUT, VPN, WIF, and auth-browser runs.
- `tests/wiremock/` — the request mocking used to pin driver behavior.
- `tests/test_coverage_report/` and `codecov.yml` — where coverage numbers come from.
- `credentials.md` already covers which credential a test run needs; extend it rather than adding a second file.
- Snowhouse tables the team queries for driver test results, flakiness, and coverage.
