# Drivers Tools Directory

How an agent drives a thing from the outside, and the data that thing produces. How a unit in this repo is built and maintained belongs in [../architecture/README.md](../architecture/README.md); a duty needing human approval belongs in [../operations/README.md](../operations/README.md).

## Catalog

| Tool | Reference | When to load |
|---|---|---|
| Bug regression coverage | [bug-regression-coverage.md](bug-regression-coverage.md) | Whether the UD has regression coverage for a specific Jira bug: a `SNOW-XXXXXX` ticket, "does UD cover this bug", "verify bug in UD". Requires a ticket key. |
| Credentials | [credentials.md](credentials.md) | Setting up `parameters.json` and `PARAMETER_PATH` before an integ or e2e run: `decode_secrets.sh`, the 1Password passphrase, the manual fallback. |
| JDBC UD tests | [jdbc-ud-tests.md](jdbc-ud-tests.md) | Building or running JDBC tests: compiling `jdbc_bridge`, `CORE_PATH`, Gradle invocations. |
| Node.js UD tests | [nodejs-ud-tests.md](nodejs-ud-tests.md) | Building or running Node.js tests: Vitest unit and e2e runs, `decode_secrets.sh`, the old-driver reference suite for comparison. |
| ODBC trace replay | [odbc-trace-replay.md](odbc-trace-replay.md) | Turning a captured ODBC trace into replay tests: sampling the trace, generating representative cases, `odbc_trace_tool`. |
| ODBC UD tests | [odbc-ud-tests.md](odbc-ud-tests.md) | Building or running ODBC tests: the C++ harness, `run.sh`, `ctest`, `DRIVER_PATH`, cmake/ninja build failures, unixodbc/iodbc setup, `libsfodbc` not found, `run_reference.sh`. |
| Old-driver coverage mapping | [old-driver-coverage-mapping.md](old-driver-coverage-mapping.md) | Reading or editing `tests/oldTestsCoverage/{jdbc,odbc,python}.yaml`: mapping an old test, listing unmapped tests, coverage gaps, reverse lookup, reconciling summary counts. |
| Python UD tests | [python-ud-tests.md](python-ud-tests.md) | Building or running Python tests: compiling `sf_core`, the hatch environment, `SKIP_CORE_BUILD`, `maturin develop`, "Couldn't load core driver dependency", `libsf_core` not found. |
