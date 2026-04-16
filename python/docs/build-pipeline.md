# Python Connector (Wrapper) Build & Release Pipeline

## Distribution via PyPI

The wrapper will be released as an **alpha** (pre-release) version of the existing
[`snowflake-connector-python`](https://pypi.org/project/snowflake-connector-python/)
package on PyPI.

**Impact on existing users:** None. By default, `pip install snowflake-connector-python`
resolves to the latest *stable* release (currently 4.4.0). Pre-release versions are
never installed unless the user explicitly opts in:

```bash
# Explicit pre-release install
pip install --pre snowflake-connector-python

# Or pin the exact alpha version
pip install snowflake-connector-python==2026.0.0a1
```

**Visibility on PyPI:** Alpha versions *are* visible on the
[Release history](https://pypi.org/project/snowflake-connector-python/#history) page
and are labelled with a "pre-release" badge, but they do **not** appear as the
"Latest version" on the project landing page. Users browsing PyPI will see the
current stable release unless they specifically look at the release history.

## Building from Source

A source distribution (`sdist`) will be uploaded to PyPI alongside the binary wheels.
Users on platforms without a pre-built wheel can install from source with:

```bash
pip install --pre snowflake-connector-python --no-binary snowflake-connector-python
```

This requires a Rust toolchain, a C compiler, and CMake on the target machine because
the sdist includes the Rust core (`sf_core`) and Cython extension sources that are
compiled during installation.

### Fallback: install directly from Git

If the PyPI pipeline is not ready by the deadline, users can install from the
repository tag directly:

```bash
pip install "snowflake-connector-python @ git+https://github.com/snowflakedb/universal-driver.git@<tag>#subdirectory=python"
```

Replace `<tag>` with the release Git tag (e.g. `python/v2026.0.0a1`).

## CI Pipeline Overview

```
             ┌───────────────────────┐
             │      PR Created       │
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │    Reduced Matrix     │  • Change detection skips unaffected jobs
             │    (build + test)     │  • Reduced matrix
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │   PR Merged to main   │
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │  Full Build & Test    │  • All OS x Python x cloud provider combos (x2 pandas)
             │       Matrix          │  • Reference connector tests
             │                       │  • Packaging tests
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │  Artifacts available  │  • Wheels per platform
             │  in GitHub Actions    │  • Source distribution
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │  Releng script        │  • Verifies builds are green
             │  verifies artifacts   │  • Creates GitHub tag
             └───────────┬───────────┘
                         │
                         ▼
             ┌───────────────────────┐
             │  Releng script        │  • Downloads wheels + sdist
             │  uploads to PyPI      │  • Uploads as alpha pre-release
             └───────────────────────┘
```

## Build & Test Matrix

After every merge to `main`, the full build and test matrix runs via the
**Python CI** GitHub Actions workflow (`.github/workflows/test-python.yml`),
producing wheel artifacts for all supported architecture/Python combinations.

### Wheel build matrix (wrapper)

| Python | Linux x86-64 | Windows ARM64 |
|--------|:------------:|:-------------:|
| 3.9    | Yes          | --            |
| 3.10   | Yes          | --            |
| 3.11   | Yes          | Yes           |
| 3.12   | Yes          | Yes           |
| 3.13   | Yes          | Yes           |
| 3.14   | Yes          | --            |

**Total: 9 wheels + 1 source distribution**

Windows ARM64 excludes Python 3.9/3.10 (no native ARM64 CPython builds in `uv`) and
3.14 (ARM64 Windows builds not yet available upstream).

### Comparison with old connector (v4.4.0)

| Python | Linux x86-64 | Linux aarch64 | macOS ARM64 | Windows x86-64 |
|--------|:------------:|:-------------:|:-----------:|:--------------:|
| 3.9    | Yes          | Yes           | Yes         | Yes            |
| 3.10   | Yes          | Yes           | Yes         | Yes            |
| 3.11   | Yes          | Yes           | Yes         | Yes            |
| 3.12   | Yes          | Yes           | Yes         | Yes            |
| 3.13   | Yes          | Yes           | Yes         | Yes            |
| 3.14   | Yes          | Yes           | Yes         | Yes            |

**Total: 24 wheels + 1 source distribution**

### Platform gap

The wrapper currently does not produce wheels for:

- **Linux aarch64** (ARM64 Linux)
- **macOS ARM64** (Apple Silicon)
- **Windows x86-64** (Intel/AMD Windows)

Users on these platforms can still install from the source distribution (see above).

## CI Infrastructure

### Previous process (old connector)

Build and wheel packaging ran on **legacy Jenkins workflows**. These were flaky and
uploaded artifacts to S3 as an intermediate store before release.

### Current process (wrapper)

All build, test, and artifact production is handled by **GitHub Actions**:

1. **`build_sf_core`** -- compiles the Rust core library (`sf_core`) once per
   platform and uploads it as a GitHub Actions artifact.
2. **`build_wheel`** -- downloads the pre-built core, builds Cython extensions, and
   produces Python wheels for each OS / Python version combination in the matrix.
   Wheels are uploaded as GitHub Actions artifacts.
3. **`packaging_tests`** -- verifies that the source distribution can be built and
   installed from source end-to-end.
4. **`python_tests`** -- installs from the built wheel and runs the full unit +
   integration test suite across OS, Python version, and cloud provider combinations.
5. **`python_tests_reference`** -- runs the same integration tests against the old
   `snowflake-connector-python` (currently v4.3.0) for comparison.
6. **`python_compare_results`** -- diffs universal vs. reference test results and
   reports regressions.

## Release Process

> **TODO (@Patryk Czajka):** Fill in the detailed release process.

When a release commit is chosen, the releng script will:

1. Create a GitHub tag.
2. Download the wheel and sdist artifacts produced by the `build_wheel` job for that
   commit.
3. Verify that the **Python CI** workflow is green.
4. Upload artifacts to PyPI.

## Binary Signing

Wheels contain unsigned native binaries (the compiled Rust core library and Cython
extensions). This is **standard practice** for Python packages on PyPI -- the vast
majority of packages with C/Rust extensions ship unsigned binaries. PyPI itself
provides integrity guarantees:

- Every uploaded file has SHA-256 / BLAKE2b-256 hashes recorded and verified on
  download.
- `pip` verifies hashes automatically when installing from PyPI.
- Uploads are authenticated via API tokens scoped to the project.

These mechanisms ensure that the artifacts users download are identical to what was
uploaded by the maintainers.
