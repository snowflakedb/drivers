# CI Test Matrix

Pairwise test matrix generation for ODBC, Python, and rust-core (`sf_core`) driver CI.

The generator combines a coverage **model** (which combinations exist) with a
**mapping** (CI metadata for each combination) consumed by the `load-{odbc,python,core}-matrix` jobs in
`.github/workflows/`.

## Layout

```
ci/test_matrix/
├── generate_matrix.py     pairwise generator + CLI entry points
├── models/                Python coverage modules (one per driver)
│   ├── __init__.py        schema documentation
│   ├── odbc.py
│   ├── python.py
│   └── core.py
├── mappings/              (OS, Arch) → CI metadata lookups
│   ├── shared.py          GHA_RUNNER (used by all drivers)
│   ├── odbc.py            ODBC_PLATFORM
│   ├── python.py          PYTHON_PLATFORM, SDIST_PY
│   └── core.py            CORE_PLATFORM
├── test_generate_matrix.py
└── generated/             gitignored; written at workflow runtime
```

**Models** (`models/*.py`) declare parameters, constraints, and the explicit
`PR_CELLS` + `JSON_CELLS` sections — the *combinatorial* shape of the matrix.
Each module exposes four top-level names (`PARAMS`, `CONSTRAINTS`, `PR_CELLS`,
`JSON_CELLS`) — see `models/__init__.py` for the schema. **Mappings**
(`mappings/*.py`) translate each `(OS, Arch)` cell into concrete CI fields:
runner label, driver/wheel artifact name, cargo flags, msvc_arch,
vcpkg_triplet, etc. The generator joins both at runtime;
`validate_mappings()` fails loud if a model allows an `(OS, Arch)` pair that
no mapping covers.

## Trigger levels

Each emitted row carries a `trigger_level`. Levels are cumulative.

| Level     | When it runs                  | Coverage                            |
|-----------|-------------------------------|-------------------------------------|
| `pr`      | Every PR                      | Explicit `PR_CELLS` only            |
| `merge`   | Merge queue + push to `main`  | `pr` cells + pairwise cover         |
| `nightly` | Scheduled nightly run         | All valid combinations              |

**Pairwise cover (the "merge" set).** A minimal set of rows in which every
(parameter A value, parameter B value) pair appears together at least once,
across every pair of parameters. With *n* parameters of size *k*, full
coverage is *kⁿ* combinations; pairwise is roughly *k²* — orders of
magnitude smaller, while still catching any bug that depends on a
two-parameter interaction.

Concretely, for `odbc.py` the parameter space is OS×Arch×Cloud. After
applying the constraints (e.g. macOS only on arm, ubuntu only on x64), 12
combinations are valid. The pairwise solver picks 4 of those — every (OS,
Arch), (OS, Cloud), and (Arch, Cloud) value combination is exercised at
least once across those 4 rows. The remaining 8 combinations only run at
`nightly`. Bugs that only appear at three- or four-way interactions (e.g.
only on macOS-arm-aws-py3.13) are not guaranteed to surface at `merge`.

GHA picks the level from `GITHUB_EVENT_NAME` (`pull_request` → pr,
`merge_group`/`push` → merge, `schedule` → nightly).

## Editing the matrix

After any edit, run `python ci/test_matrix/generate_matrix.py --all` and
review the diff in `ci/test_matrix/generated/` (it's gitignored, so the diff
is local-only — used only to sanity-check the change). Tests live in
`test_generate_matrix.py`; run `python -m pytest ci/test_matrix/`.

### Add a PR cell

Append a dict to `PR_CELLS` in the relevant `models/<driver>.py`. Every
declared parameter must be present (the loader validates this), and the
combination must satisfy every constraint — a violating cell warns at
generate time and is silently dropped from the output.

### Add a constraint

Each model declares a single `is_valid(c)` block-list function in
`models/<driver>.py`: every `if` line names a forbidden combo with positive
`==` matching and `return False`; the function falls through to `return True`
for everything else.

```python
def is_valid(c):
    if c["OS"] == "windows" and c["Arch"] == "arm":
        # No CPython 3.10 build for Windows-on-ARM.
        if c["PyVersion"] == "3.10":      return False
        # No pyarrow win_arm64 wheel.
        if c["HatchEnv"] == "test-pandas": return False
    return True


CONSTRAINTS = [is_valid]
```

To add a new restriction, append an `if ...: return False` line with a
`# comment` explaining the real-world reason (no upstream wheel, runner
unavailable, etc.). Block-list semantics mean **new PARAMS values are
accepted automatically** — adding `"3.15"` to `PARAMS["PyVersion"]`
extends the matrix without touching `is_valid`.

The shape is enforced by `BlockListShapeTests` in
`test_generate_matrix.py`: every `return` must be a `False`/`True` bool
literal (no `return <expression>`), and `return True` may appear only as
the final statement of `is_valid`. This blocks the two accidental drift
patterns: allow-list creep (`return c["X"] in (...)`) and early
`return True` short-circuits.

### Reordering `PARAMS`

`PARAMS` order affects which specific cells land in the merge-level
pairwise cover (greedy tie-breaking is "first wins" over
`itertools.product`-order candidates). Coverage and row content are
order-independent, and nightly stays the full constraint-valid cartesian
product, but the merge cell selection shifts. Pick an order and keep it
stable; the existing `OS, Arch, Cloud, …` convention is fine — don't
shuffle it without a reason.

### Add a JSON `result_format` variant

Append a dict to `JSON_CELLS["pr"]` / `JSON_CELLS["merge"]` /
`JSON_CELLS["nightly"]` in the model file. Each listed row gets duplicated
with `result_format="json"` at the named trigger level. `core.py` has no
JSON axis (sf_core's JSON coverage runs through ODBC and Python). The total
merge-level JSON count is pinned by `JsonVariantRegressionTests`.

### Add a new platform lane (existing driver)

1. **Model** — add the OS/Arch values to `PARAMS` in `models/<driver>.py`.
   Add constraints if the new lane only supports a subset of clouds, py
   versions, etc.
2. **Mapping** —
   - If the `(OS, Arch)` pair is new across drivers, add it to `GHA_RUNNER`
     in `mappings/shared.py`.
   - Add a row to `mappings/<driver>.py`'s `<DRIVER>_PLATFORM` dict with all
     fields the driver needs (`driver_lib` + `driver_artifact` for ODBC,
     `wheel_artifact` + `wheels` for Python, `cargo_flags` + `coverage` +
     `cache_key` for core; see existing rows).
3. If ODBC and the lane should produce a built artifact, add a matching
   `build_odbc_driver` matrix entry in `.github/workflows/test-odbc.yml` and
   set `driver_artifact` to that artifact's name. Same for Python wheels via
   `_build-python-wheels.yml`.
4. Regenerate and run pytest. `validate_mappings()` will refuse if any
   required mapping field is missing.

### Add a new driver

1. Create `models/<new>.py` exposing `PARAMS`, `CONSTRAINTS`, `PR_CELLS`, and
   `JSON_CELLS` (see `models/__init__.py` for the schema).
2. Create `mappings/<new>.py` with `<NEW>_PLATFORM` (or whatever the driver
   needs); re-export it from `mappings/__init__.py`.
3. In `generate_matrix.py`, add a `_build_<new>_row` builder and route to it
   from `generate()`. Extend `validate_mappings()` to enforce the new
   driver's required fields.
4. Add the driver name to the `--driver` choices in `main()`. Add a
   `load-<new>-matrix` job and a consumer job in
   `.github/workflows/test-<new>.yml`.

## CLI

```
python ci/test_matrix/generate_matrix.py [--driver odbc|python|core | --all]
python ci/test_matrix/generate_matrix.py --driver <D> --event <NAME> --emit-active
```

| Flag             | Description                                                                |
|------------------|----------------------------------------------------------------------------|
| `--driver <D>`   | Generate `<D>-gha.json` for one driver.                                    |
| `--all`          | Regenerate every driver in one shot.                                       |
| `--event <NAME>` | GHA event name for `--emit-active`.                                        |
| `--emit-active`  | Print `matrix=<json>` for rows active at the level implied by `--event`.   |

`--driver` and `--all` are mutually exclusive; `--emit-active` requires both
`--driver` and `--event`.

## Row schema

The complete list of keys per row, including which are required and on which
driver, is documented next to the row builders in `generate_matrix.py`
(`_build_gha_row`, `_build_core_row`). Highlights:

- `name`, `os` (GHA runner label, not raw OS), `trigger_level` — present on
  every row.
- `cloud_provider`, `py`, `hatch_env`, `driver_*`, `wheel_artifact` — odbc
  and/or python only; absent on core.
- `cargo_flags`, `cargo_target`, `coverage`, `cache_key`, `msvc_arch` — core
  only.
- `result_format` — set to `"json"` only on cells from `JSON_CELLS`.
