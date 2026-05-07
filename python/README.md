# ⚠️ PRIVATE PREVIEW DISCLAIMER ⚠️

**IMPORTANT:** This release contains a **Private Preview** version of the Snowflake DB driver. By downloading or using this software, you acknowledge and agree to the following conditions:

- **Restricted Access:** Usage of this driver is strictly limited to participants who are actively enrolled in the official Private Preview program for this specific driver. If you have not been explicitly invited and authorized to participate in this preview, do not download, install, or use this software.
- **Testing Environments Only:** For authorized Private Preview partners, this driver is provided solely for evaluation, feedback, and testing purposes. It must be deployed exclusively in isolated, non-production environments.
- **"As-Is" Software:** As a preview release, this driver is still under active development. It may contain bugs, lack features, or undergo significant breaking changes before general availability. It is provided "as-is" without any warranties, service level agreements (SLAs), or official support commitments.

## Development

### Prerequisites
- Python 3.10+
- [uv](https://docs.astral.sh/uv/) package manager
- [Hatch](https://hatch.pypa.io/) build tool
- Rust toolchain (for building core library)
- Credentials: `../parameters.json` (see main [README.md](../README.md) for setup instructions)

### Setup

Install uv and Hatch:
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Hatch
uv tool install hatch
```

**Note:** The Rust core library is built automatically during the build process via a custom Hatch build hook. You don't need to build it manually.

### Environment Variables

**IMPORTANT:** Set up required environment variables before running tests:

```bash
export PARAMETER_PATH="$(pwd)/../parameters.json"
```

## Hatch Environments

This project uses [Hatch](https://hatch.pypa.io/) to manage development environments. There are two primary environments for running tests, each designed for a different use case:

### `dev` Environment (for local development)

The `dev` environment is designed for **human developers** during active development. It installs the package from sources in **editable mode**, meaning any code changes are immediately reflected without reinstallation.

**Key characteristics:**
- Installs from sources (`skip-install = false`)
- Editable mode enabled (`dev-mode = true`)
- Changes to source code are immediately available
- Supports Python matrix: 3.10, 3.11, 3.12, 3.13, 3.14

**Usage:**
```bash
# Run all tests with the dev environment
hatch run dev:all

# Run specific test types
hatch run dev:unit              # Unit tests only
hatch run dev:integ             # Integration tests only
hatch run dev:e2e               # End-to-end tests only

# Run with coverage
hatch run dev:all-cov

# Run with specific Python version
hatch run dev.py3.12:all

# Pass additional pytest arguments
hatch run dev:all -k test_connection --maxfail=1
```

### `test` Environment (for CI/CD pipelines)

The `test` environment is designed for **CI systems** to test the end-to-end software development lifecycle (SDLC). It does **not** install from sources - instead, it expects a pre-built wheel to be installed, simulating how end users would install and use the package.

**Key characteristics:**
- Skips source installation (`skip-install = true`)
- No editable mode (`dev-mode = false`)
- Requires explicit wheel installation via `install-wheel` script
- Tests the actual built artifact, not source code

**Usage:**
```bash
# First, build the wheel
hatch build

# Then install and test with the test environment
hatch run test:install-wheel
hatch run test:all

# Or with specific Python version
hatch run test.py3.12:install-wheel
hatch run test.py3.12:all
```

### Environment Comparison

| Aspect | `dev` | `test` |
|--------|-------|--------|
| **Purpose** | Local development | CI/CD pipelines |
| **Installs from** | Source (editable) | Pre-built wheel |
| **Code changes** | Immediately reflected | Requires rebuild |
| **Tests** | Source code | Built artifact |
| **Use case** | Developer workflow | E2E SDLC validation |

### Other Environments

```bash
# Code quality checks
hatch run precommit:check        # Run all checks (format, lint, type)
hatch run precommit:fix          # Auto-fix formatting and linting issues

# Reference connector tests (for compatibility testing)
PYTHON_REFERENCE_DRIVER_VERSION=3.17.2 hatch run reference:test
```

## References

- [PEP 249 - Python Database API Specification v2.0](https://peps.python.org/pep-0249/)
- [Python Database API Specification v2.0](https://www.python.org/dev/peps/pep-0249/) 