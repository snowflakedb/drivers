---
description: Authoring rules for drivers operations references — file structure, registration, and discovery.
no-pointer: true
---

# Contributing to Operations References

Follow the shared rules in [../../CONTRIBUTING.md](../../CONTRIBUTING.md), plus this section's own rules. Every rule here obeys [../../CONTRIBUTING.md § The laws govern this file too](../../CONTRIBUTING.md#the-laws-govern-this-file-too).

## File structure

- **When to run this** — the trigger, and the state that must already hold.
- **Approval** — who signs off, and at which step. Mark the step where an agent stops and waits.
- **Steps** — numbered, each naming the surface it acts on and the observation that confirms it took effect.
- **Rollback** — how to undo each step that changes state, or a statement that a step cannot be undone.
- **Verification** — what to check afterwards, and the values that mean it worked.

A report file replaces **Approval** and **Rollback** with the schedule, the audience, and where the output is posted.

## Registration

- Place the file at `kb/references/operations/<operation>.md`.
- Add a row to the catalog in [`README.md`](README.md), keeping the table's sort order.
- Run `sf ai rules build`, then `sf ai rules lint`.

## Discovery prompts

- `.cursor/commands/odbc-weekly-report.md` — the recurring report and the data behind it.
- `.github/workflows/security-signoff.yml`, `security-label.yml`, and `environment-cleanup.yml` — gated and scheduled actions.
- `.github/workflows/mirror*.yml` and `NOMIRROR` — what leaves this repo for a public one, and the privacy check on it.
- The release build workflows for the JDBC jars, ODBC packages, and Python wheels.
- `.claude/rules/pr-creation-draft-default.md` — the approval posture the team expects of an agent.
