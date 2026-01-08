# sf_mini_core

A minimal Rust dynamic library used to validate dynamic library loading in existing Snowflake drivers.

## Background

This library serves as a proof-of-concept for loading Rust-based extensions into Snowflake's existing drivers (ODBC, JDBC, Python, etc.). By shipping this minimal library alongside existing drivers, we can:

1. Generate telemetry on dynamic library loading across different platforms and environments
2. Validate the loading mechanism before introducing more complex Rust functionality
3. Identify compatibility issues early in the development cycle

The library exports a single C-compatible function (`sf_core_full_version`) that returns the version string, providing a simple way to verify the library loaded successfully.

## Building & Packaging

Use the packaging script to build the library and generate the C header:

```bash
# Set the target platform (see script for all options)
export PLATFORM=macos-aarch64  # or linux-x86_64-glibc, windows-x86_64, etc.

# Run the package script
./scripts/package_minicore.sh
```

This will:
1. Generate the C header using cbindgen
2. Build both dynamic and static library variants
3. Create a distributable archive in `build/`

Output files in `build/`:
- `sf_mini_core.h` - C header file
- `libsf_mini_core.dylib` / `.so` / `.dll` - Dynamic library
- `libsf_mini_core_static.a` / `.lib` - Static library

### Manual Build (without packaging)

```bash
# Build just the dynamic library
cargo build --release -p sf_mini_core

# Generate the header manually
cbindgen --config sf_mini_core/cbindgen.toml --crate sf_mini_core > sf_mini_core.h
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

