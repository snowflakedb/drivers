# Drivers Concepts Directory

Cross-cutting behavior spanning two or more wrappers or crates. A fact about one wrapper or one crate on its own belongs in [../architecture/README.md](../architecture/README.md) instead.

## Catalog

| Concept | Reference | When to load |
|---|---|---|
| Behavior differences | [behavior-differences.md](behavior-differences.md) | A deliberate divergence from the legacy driver: `BehaviorDifferences.yaml`, `is_breaking_change`, `api_incompatibility`, the `behavior-differences-validator` hook, a duplicate-ID rejection, or `status_rationale` required on ODBC. |
| Connection parameters | [connection-parameters.md](connection-parameters.md) | Adding or changing a connection parameter; `sf_params_spec`, `param_defs_export`, a regenerated `connection_config.py` or `sf_core_python.pyi`, or an edit to a generated file being reverted. |
| Core-and-bridge split | [core-and-bridge-split.md](core-and-bridge-split.md) | How a call reaches `sf_core` from a wrapper: `database_driver_v1.proto`, `RustTransport`, `WrapperPresets`, the JNI / PyO3 / N-API boundary, a `cdylib` the host runtime loads, or `CORE_PATH` / `DRIVER_PATH` load failures. |
| Old-driver coverage attribution | [old-driver-coverage-attribution.md](old-driver-coverage-attribution.md) | Whether a legacy test's behavior is covered at all: `tests/oldTestsCoverage/`, an empty `ud_tests` list, unmapped legacy cases, or drifted `summary` counts. |
