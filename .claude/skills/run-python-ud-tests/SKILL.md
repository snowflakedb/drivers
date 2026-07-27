---
name: run-python-ud-tests
description: >
  Runbook for building and running Python Universal Driver (UD) tests. Use
  when you need to compile the Rust core (sf_core), set up the hatch
  environment, and execute Python UD unit/integ/e2e tests. Also use for:
  "RuntimeError: Couldn't load core driver dependency", "libsf_core not
  found", SKIP_CORE_BUILD, "hatch run dev:unit", "maturin develop", or
  running an adhoc script when credentials or the compiled core are
  unavailable.
---

## Python UD Test Runner

All commands run from the **repo root** unless stated otherwise.

---

## Prerequisites

- Python 3.10+
- `uv` — `curl -LsSf https://astral.sh/uv/install.sh | sh`
- `hatch` — `uv tool install hatch`
- Rust toolchain with `cargo` on PATH

---

## Step 1 — Credentials (required for integ/e2e, not for unit)

Unit tests do **not** need `parameters.json` but still require the compiled
Rust core (see Step 2).

For integ/e2e: ensure `parameters.json` exists at the repo root and
`PARAMETER_PATH` is exported. Full setup procedure:

@.claude/rules/ud-credentials.md

---

## Step 2 — Compile the Rust core

The Python connector loads `libsf_core` at import time. **Even unit tests
fail** if the library is absent because `c_api.py` is imported transitively
from `connector/__init__.py`.

Check whether the core is already built:

```bash
# macOS
ls python/src/snowflake/connector/_core/*.dylib 2>/dev/null && echo "present" || echo "missing"
# Linux
ls python/src/snowflake/connector/_core/*.so    2>/dev/null && echo "present" || echo "missing"
```

If missing, the first `hatch run dev:…` command will build it automatically
via `python/hatch_build.py`. To pre-build manually (faster iteration):

```bash
# Debug build — faster compile, used for local dev:
cargo build --package sf_core

# Release build — matches CI:
cargo build --release --package sf_core
```

`hatch_build.py` then copies the artifact to
`python/src/snowflake/connector/_core/` when the hatch env is (re)created.

**Key env vars for the build step:**

| Variable | Default | Effect |
|---|---|---|
| `SKIP_CORE_BUILD` | `1` in dev env | Set to `0` to force rebuild on next `hatch run` |
| `CORE_CARGO_TARGET_DIR` | temp dir | Stable dir for incremental builds / CI caching |
| `SF_PERF_METRICS` | unset | Set `1` to compile with `--features perf_timing` |

---

## Step 3 — Run tests

All `hatch run` commands are run from `python/`:

```bash
cd python
```

### Unit tests (no live connection, core required)

```bash
hatch run dev:unit                       # all unit tests
hatch run dev:unit -k <pattern>          # filter by name
hatch run dev:unit tests/unit/test_foo.py::TestClass::test_method
```

### Integration tests (core + credentials required)

```bash
hatch run dev:integ
hatch run dev:integ -k <pattern>
```

### E2E tests (core + credentials required)

```bash
hatch run dev:e2e
hatch run dev:e2e -k <pattern>
```

> **Note:** `pyproject.toml` hardcodes `-n auto` (xdist) and `testpaths = ["tests"]` in
> `addopts`, so passing a file path alone does **not** restrict which tests run — pytest
> still collects the full suite. To run a single test:
>
> ```bash
> hatch run dev:e2e \
>   --ignore=tests/e2e/pandas \
>   -k "test_name" \
>   --override-ini="addopts=-vv --strict-markers --strict-config -m 'not flaky'" \
>   -p no:randomly
> ```
>
> - `--ignore=tests/e2e/pandas` — pandas is not installed in the `dev` env; without
>   this, collection fails with 15 `ModuleNotFoundError` errors.
> - `--override-ini` — strips out `-n auto` so xdist doesn't spread execution across
>   all 810 tests.
> - `-k "test_name"` — selects the specific test by name substring.

### All tests

```bash
hatch run dev:all
hatch run dev:all -k <pattern>
hatch run dev:all-cov                    # with HTML + XML coverage report
```

### Pandas-specific tests

```bash
hatch run dev:pandas-cov
```

### Pin a Python version

```bash
hatch run dev.py3.12:unit
hatch run dev.py3.12:all
```

### Test with JSON result format (instead of default Arrow)

```bash
QUERY_RESULT_FORMAT=JSON hatch run dev:all
```

---

## Troubleshooting

### `RuntimeError: Couldn't load core driver dependency`

The compiled library is missing from `_core/`. Fix:

```bash
cd python
hatch env remove dev          # wipe stale env
hatch run dev:unit            # recreates env, triggers build hook
```

Or pre-build and let the hook copy:

```bash
cargo build --package sf_core
cd python && hatch env remove dev && hatch run dev:unit
```

### Force full rebuild from scratch

```bash
hatch env prune               # removes ALL hatch environments
cargo clean --package sf_core # optional: clean Rust artifacts too
hatch run dev:unit            # rebuilds everything
```

### `SKIP_CORE_BUILD=1` is set and core is absent

Unset it before running:

```bash
SKIP_CORE_BUILD=0 hatch run dev:unit
```

---

## Temporary regression test pattern

When writing a temporary regression test for a bug ticket, place it in the
standard test directories and run it via hatch — **not** as a standalone
script. This ensures it runs in the real test environment with the correct
imports and fixtures.

**For pure-logic tests (no live connection)** — place in `tests/unit/`:

```python
# Temporary regression test: SNOW-XXXXXX
# <one-line description of what bug this guards against>
import re
import pytest
from snowflake.connector._internal.write_pandas_operation import generate_temp_name


class TestRegressionSnowXXXXXX:
    def test_unique_names_across_many_calls(self):
        names = [generate_temp_name("STAGE") for _ in range(10_000)]
        assert len(set(names)) == len(names), "Duplicate temp stage names detected"

    def test_name_format(self):
        name = generate_temp_name("STAGE")
        assert re.match(r"^__WRITE_PANDAS_STAGE_[0-9a-f]{16}$", name)
```

Run via hatch:
```bash
cd python
hatch run dev:unit -k TestRegressionSnowXXXXXX
```

**For tests requiring a live connection** — place in `tests/integ/`:
```bash
hatch run dev:integ -k TestRegressionSnowXXXXXX
```

**If the Rust core is not compiled** and the test only exercises logic that
can be replicated without importing the full connector (no cursor, no
connection, no result-set parsing), you may inline the function-under-test
directly in the test file to avoid the `c_api.py` import chain. Mark clearly:
```python
# Inlined from snowflake.connector._internal.write_pandas_operation
# because core not yet compiled — remove inline once core is built
import secrets
def generate_temp_name(prefix: str) -> str:
    return f"__WRITE_PANDAS_{prefix}_{secrets.token_hex(8)}"
```
This is a last resort — prefer building the core first (see Step 2 above).
