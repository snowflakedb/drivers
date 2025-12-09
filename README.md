# universal-driver

[![codecov](https://codecov.io/gh/acme/universal-driver/branch/main/graph/badge.svg)](https://codecov.io/gh/acme/universal-driver)

# Running Tests

This project contains multiple test suites across different driver implementations. Before running any tests, you'll need to set up common credentials and build the required components.

## Prerequisites

### 1. Decode Secrets

All integration tests require access to Snowflake credentials. To set up the required `parameters.json` file:

```bash
# Install 1Password CLI if not already installed
# Then decode the encrypted parameters file:
./scripts/decode_secrets.sh
```

This will create a `parameters.json` file in the project root containing test credentials.

**Alternative:** If you don't have 1Password CLI installed you can provide encryption password via environment variable:
```bash
PARAMETERS_SECRET=<encryption-password> ./scripts/decode_secrets.sh 
```

**Alternative:** If you want provide different credentials that the standard ones, you can provide parameters.json yourself:

```json
{
  "testconnection": {
    "SNOWFLAKE_TEST_ACCOUNT": "your-account",
    "SNOWFLAKE_TEST_USER": "your-username",
    "SNOWFLAKE_TEST_PASSWORD": "your-password",
    "SNOWFLAKE_TEST_DATABASE": "your-database",
    "SNOWFLAKE_TEST_SCHEMA": "your-schema",
    "SNOWFLAKE_TEST_WAREHOUSE": "your-warehouse",
    "SNOWFLAKE_TEST_HOST": "your-host.snowflakecomputing.com",
    "SNOWFLAKE_TEST_ROLE": "your-role"
  }
}
```

### 2. Build Core Components

The tests require the Rust core library to be built:

```bash
# Build all Rust components
cargo build

# Or build specific packages
cargo build --package sf_core
cargo build --package jdbc_bridge
```

## Driver-Specific Testing

Each driver implementation has its own testing setup and requirements. See the individual README files for detailed instructions:

- **Python (PEP 249)**: See [python/README.md](python/README.md)
- **Rust Core**: See [sf_core/README.md](sf_core/README.md)
- **ODBC**: See [odbc/README.md](odbc/README.md)
- **JDBC**: See [jdbc/README.md](jdbc/README.md)

## Test Validation

The project includes a test format validator that ensures Gherkin feature files have corresponding implementations:

```bash
# Run validator
./tests/tests_format_validator/run_validator.sh

# Or run directly
cd tests/tests_format_validator/
cargo run
```

## CI/Local Environment Differences

- **Local**: Tests auto-build missing Rust components
- **CI**: Requires pre-built components and explicit environment variables
- **Docker**: Reference tests use containerized official drivers

## Troubleshooting

1. **Missing parameters.json**: Run `./scripts/decode_secrets.sh` or create manually
2. **Missing Rust library**: Run `cargo build --package sf_core`

## License
Copyright (c) Snowflake Inc. All rights reserved.

Licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
