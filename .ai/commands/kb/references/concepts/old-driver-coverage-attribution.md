---
description: How a legacy driver test is attributed to its Universal Driver equivalent — the tests/oldTestsCoverage YAML shape, what counts as covered, and why an empty ud_tests list is the unit of outstanding work.
no-pointer: true
---

# Attributing Legacy Driver Coverage to the Universal Driver

The Universal Driver must not lose a behavior its predecessor tested. That obligation is tracked as an explicit mapping from each legacy test case to the UD test cases that cover it, kept in this repo rather than in the legacy repos.

## Where the mapping lives

`tests/oldTestsCoverage/`, one file per legacy driver: `jdbc.yaml`, `odbc.yaml`, `python.yaml`. There is no Node.js file; its comparison runs against the old-driver reference suite instead — see [../tools/nodejs-ud-tests.md](../tools/nodejs-ud-tests.md).

Each file has the same shape:

```yaml
title: <driver> Driver Test Coverage
description: Mapping of old <driver> driver tests to new Universal Driver test suite.
summary:
  total_tests: <int>
  total_test_files: <int>
tests:
  <legacy test file path>:
  - test_name: <legacy test case name>
    ud_tests:
    - <UD test file path><separator><case>
```

The key under `tests:` is the legacy test's **file path** in its own repo, and each entry beneath it is one named case in that file.

## What "covered" means

`ud_tests` is the whole judgement, and it is a **flat list of strings**, each naming one UD test as a path plus the case within it. A non-empty list asserts those UD tests exercise the same behavior as the legacy case; an **empty list means unmapped**, and the set of empty lists is the outstanding work. Because the claim is a list of names rather than a tool's output, treat a populated `ud_tests` as a claim to verify at the assertion level, not as proof.

### The separator differs per file

There is no repo-wide separator. Each coverage file uses its own, consistently, and the choice follows the **file** rather than the language of the UD test being named — `python.yaml` uses `::` even for a Rust path, and `odbc.yaml` uses ` - ` even for a Rust path.

| File | Separator | Example entry |
|---|---|---|
| `odbc.yaml` | ` - ` (space, hyphen, space) | `sf_core/tests/e2e/authentication/native_okta.rs - vpn_should_authenticate_using_native_okta` |
| `jdbc.yaml` | `#` | `jdbc/src/test/java/.../SnowflakeDriverTest.java#testAcceptsInvalidURL` |
| `python.yaml` | `::` | `sf_core/src/config/path_resolver.rs::tests::test_get_config_paths_default` |

When adding an entry, match the separator already used in that file. Writing a `#` into `odbc.yaml` or `python.yaml` produces an entry that matches nothing else in it.

### Which keys occur

`test_name` and `ud_tests` occur on every entry in all three files. `notes` occurs **only in `python.yaml`**, and there on a minority of entries; `odbc.yaml` and `jdbc.yaml` contain none. A richer per-entry schema — `status` (`unmapped`/`partial`/`mapped`/`not-applicable`), a `gaps` list, and a `jira` key — is described in [../tools/old-driver-coverage-mapping.md](../tools/old-driver-coverage-mapping.md) as the target, and none of those keys is present in any of the three files. Read a file for what it holds, not for that target shape, and do not treat a missing `status` as malformed.

`summary.total_tests` and `total_test_files` are maintained alongside the entries and can drift from the actual entry count. Reconcile them against the parsed file rather than trusting them.

## Constraints

- The legacy repos are not in this repo and are not precloned. Resolve one through a local checkout or the GitHub MCP tools, per [../architecture/repos/legacy-drivers.md](../architecture/repos/legacy-drivers.md).
- Coverage attribution answers "is this behavior tested at all". It is not code-coverage measurement (line, branch, Jacoco, LCOV), and it is not the record of deliberate divergence — that is [behavior-differences.md](behavior-differences.md).
- Trimming a legacy reference test from scope is governed by `.claude/rules/old-driver-reference-coverage-trim.md`.

The working procedure for reading and editing these files is [../tools/old-driver-coverage-mapping.md](../tools/old-driver-coverage-mapping.md).
