# Drivers Architecture Directory

The build-and-maintain view of this repo: what each shipped driver and internal crate is, the legacy repos the UD replaces, and how to develop the code. Behavior that spans several of these units belongs in [../concepts/README.md](../concepts/README.md); driving a thing from the outside belongs in [../tools/README.md](../tools/README.md).

No reference sits directly in this directory. Each one lives in a subsection, and this catalog groups them.

## Catalog

### `components/` — what the repo ships to a user

| Component | Reference | When to load |
|---|---|---|
| The shipped drivers | [components/drivers.md](components/drivers.md) | Which wrappers exist and what each ships: directory, build toolchain, release artifact, and whether it is released, tested-only, or still a placeholder. |

### `libraries/` — crates imported by the components, not run on their own

| Library | Reference | When to load |
|---|---|---|
| `sf_core` | [libraries/sf-core.md](libraries/sf-core.md) | The crate holding all driver behavior: its module surface (auth, rest, chunks, file_manager, logging, sensitive), the protobuf and `c_api` boundaries, and where its tests live. |
| The bridge crates | [libraries/bridges.md](libraries/bridges.md) | `jdbc_bridge`, `python_bridge`, `nodejs_bridge`: what they own, the JNI / PyO3 / N-API mechanism each uses, and why ODBC has none. |
| Supporting crates | [libraries/support-crates.md](libraries/support-crates.md) | `error_trace`, the protobuf codegen pair, the connection-parameter spec pair, and `sf_mini_core` — the loading probe shipped with the legacy drivers. |

### `repos/` — other repos of interest, not owned here

| Repo | Reference | When to load |
|---|---|---|
| Legacy driver repos | [repos/legacy-drivers.md](repos/legacy-drivers.md) | Locating the driver the UD replaces: the `snowflakedb/snowflake-*` identifiers, the local paths a checkout is expected at, and the GitHub MCP fallback when there is none. |

### `best-practices/` — how to develop this code

| Topic | Reference | When to load |
|---|---|---|
| Adding tests | [best-practices/adding-tests.md](best-practices/adding-tests.md) | **Placement only** — which directory and which tier (unit, integ, e2e) a new test belongs in for a given wrapper, and whether a Gherkin feature scenario is required with it. Not how to write its body, and not how to run it. |
| End-of-task review | [best-practices/end-of-task-review.md](best-practices/end-of-task-review.md) | Before calling a change done or opening a PR: commit-scope drift, loose assertions, refactor-stale comments, error-string changes needing a callout, self-sabotaging fixtures, dead-code helpers. |
| Formatting | [best-practices/formatting.md](best-practices/formatting.md) | Which formatter to run for the subsystems a change touched, before committing; plus the `format!`/`assert!` inline-variable style `cargo fmt` will not apply. |
| Gherkin datatypes | [best-practices/gherkin-datatypes.md](best-practices/gherkin-datatypes.md) | Writing a Gherkin feature scenario for datatype coverage, and the conventions a feature file follows. |
| Graphite PR workflow | [best-practices/graphite-pr-workflow.md](best-practices/graphite-pr-workflow.md) | Creating a commit or PR here with the Graphite CLI, and the SNOW-ticket title convention. |
| JDBC test review | [best-practices/jdbc-test-review.md](best-practices/jdbc-test-review.md) | Judging a JDBC/Java test for correctness and flakiness, and the flakiness patterns to reject. |
| ODBC test generation | [best-practices/odbc-test-generation.md](best-practices/odbc-test-generation.md) | Writing a new ODBC test: the C++ layout, `SQLBindCol` and datatype-conversion conventions, and the pre-landing checklist. |
| ODBC test review | [best-practices/odbc-test-review.md](best-practices/odbc-test-review.md) | Judging a C++ test under `odbc_tests/`: best practices, anti-patterns, correctness checks. |
| Rust error handling | [best-practices/rust-error-handling.md](best-practices/rust-error-handling.md) | Writing or changing error handling in the Rust crates, including conversions across the FFI boundary. |
| Test generation | [best-practices/test-generation.md](best-practices/test-generation.md) | Writing the body of an integration or e2e test, proving its assertions actually fail, and **the rules for running tests** — including `PARAMETER_PATH`, without which a test that connects to Snowflake will not run. |
