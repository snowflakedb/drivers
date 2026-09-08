# Per-driver evidence paths

This skill lives at repo-root `.claude/skills/` because every language wrapper
uses the same review workflow. Look up evidence only for drivers in the
classification step (plus all wrappers when `sf_core/` is in the diff).

## Path table

| Driver | Diff paths | BD file | Coverage index | Old-contract source |
|---|---|---|---|---|
| Node.js | `nodejs/`, `nodejs_bridge/` | `nodejs/BehaviorDifferences.yaml` | none yet | In-tree `nodejs/tests/_old-driver-reference/` + current Node.js tests |
| Python | `python/`, `python_bridge/` | `python/BehaviorDifferences.yaml` | `tests/oldTestsCoverage/python.yaml` | Index + current Python tests; no in-tree frozen suite |
| JDBC | `jdbc/`, `jdbc_bridge/` | `jdbc/BehaviorDifferences.yaml` | `tests/oldTestsCoverage/jdbc.yaml` | Index + current JDBC tests; no in-tree frozen suite |
| ODBC | `odbc/`, `odbc_tests/` | `odbc_tests/BehaviorDifferences.yaml` | `tests/oldTestsCoverage/odbc.yaml` | Index + current ODBC tests; no in-tree frozen suite |
| .NET | `dotnet/` | none yet | `tests/oldTestsCoverage/dotnet.yaml` | Index only; say there is no BD file |

## `sf_core/`

Treat `sf_core/` as a shared contract. For each public surface in the core diff,
review **all** wrappers (JDBC, Python, ODBC, Node.js, and .NET if indexed) —
not only wrappers that also appear in this diff.

An empty `ud_tests` list is inventory data, not evidence that this diff creates
a gap. Search only rows that match the changed shared surface, and apply the
same relevance gate as a wrapper-local review.

## Missing old source

If the old driver test file is not in this repo, keep running: BD YAML,
coverage YAML, and new-driver tests. Use `insufficient_evidence` instead of
inventing a behavior difference or a Jira key.
