---
description: How a connection parameter is defined once in sf_params_spec and reaches the core — the param_defs_export binary, the generated Python connection_config.py (the only generated wrapper view today), and why generated files are never hand-edited.
no-pointer: true
---

# Connection Parameters from a Single Definition

A connection parameter is defined once, in Rust, in `sf_params_spec`. That definition is what the core honours, so hand-adding a parameter to a wrapper produces a driver that accepts an option the core ignores.

**Only Python has a generated view.** `param_defs_export` has exactly one consumer — the `generate-connection-config` pre-commit hook, which writes `python/src/snowflake/connector/connection_config.py`. There is no generated connection-config for JDBC, ODBC, Node.js, or .NET; those wrappers surface parameters through their own hand-written surfaces. Do not assume adding a parameter to the spec updates them.

## How it works

`sf_params_spec` (`sf_params_spec/src/lib.rs`) holds the parameter definitions. `sf_params_codegen` depends on it and ships the binary **`param_defs_export`** (`sf_params_codegen/src/main.rs`), which emits a wrapper-facing view of that spec.

For Python, the pre-commit hook `generate-connection-config` runs:

```
cargo run -p sf_params_codegen --bin param_defs_export > python/src/snowflake/connector/connection_config.py
```

then formats the result through `hatch run precommit:fix` and re-stages it. A second hook, `generate-python-stubs`, regenerates `python/src/snowflake/connector/_core/sf_core_python.pyi` from the Rust source via `python_stub_gen`.

## Constraints

- **`python/src/snowflake/connector/connection_config.py` and `sf_core_python.pyi` are generated.** Edit `sf_params_spec` and let the hooks regenerate them; a direct edit is reverted by the next commit that touches the spec.
- The hooks run on commit, so a parameter change and its generated output land together. A PR showing a spec change with no regenerated file means the hook did not run.
- Parameters reach the core as part of the protobuf payload described in [core-and-bridge-split.md](core-and-bridge-split.md); `WrapperPresets` is how a bridge declares the per-wrapper defaults applied on top of the shared spec.
