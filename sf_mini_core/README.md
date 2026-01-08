# sf_mini_core

A minimal Rust dynamic library used to validate dynamic library loading in existing Snowflake drivers.

## Background

This library serves as a proof-of-concept for loading Rust-based extensions into Snowflake's existing drivers (ODBC, JDBC, Python, etc.). By shipping this minimal library alongside existing drivers, we can:

1. Generate telemetry on dynamic library loading across different platforms and environments
2. Validate the loading mechanism before introducing more complex Rust functionality
3. Identify compatibility issues early in the development cycle

The library exports a single C-compatible function (`sf_core_full_version`) that returns the version string, providing a simple way to verify the library loaded successfully.

## Building

```bash
# Build the dynamic library
cargo build --release -p sf_mini_core

# The output will be in target/release/:
#   - libsf_mini_core.dylib (macOS)
#   - libsf_mini_core.so (Linux)
#   - sf_mini_core.dll (Windows)
```

## Generated Header

The C header is auto-generated using cbindgen during the build process. After building, the header can be found at `target/sf_mini_core.h`.

To manually generate the header:

```bash
cbindgen --config sf_mini_core/cbindgen.toml --crate sf_mini_core --output target/sf_mini_core.h
```

## Usage from C/C++

```c
#include "sf_mini_core.h"
#include <stdio.h>

int main() {
    const char* version = sf_core_full_version();
    printf("sf_mini_core version: %s\n", version);
    return 0;
}
```

## API

### `sf_core_full_version`

```c
const char* sf_core_full_version(void);
```

Returns a pointer to a static null-terminated string containing the library version. The returned pointer is valid for the lifetime of the program and must not be freed by the caller.

