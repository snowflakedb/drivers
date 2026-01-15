# Snowflake Go Driver (Universal)

This is the universal Go driver for Snowflake, implementing the standard `database/sql` interface.

## Features

- **database/sql Compatible**: Works with Go's standard database interface
- **Multiple Backends**: Supports both WASM and native (CGO) backends
- **Arrow Results**: Efficient Arrow-based result handling
- **JWT Authentication**: Full support for key-pair (JWT) authentication
- **DSN Parsing**: Compatible with gosnowflake DSN format

## Installation

```bash
go get github.com/snowflakedb/universal-driver/go/snowflake
```

## Quick Start

```go
package main

import (
    "database/sql"
    "log"

    _ "github.com/snowflakedb/universal-driver/go/snowflake"
)

func main() {
    // Connect using DSN
    dsn := "user@account/database/schema?warehouse=wh&role=role"
    db, err := sql.Open("snowflake", dsn)
    if err != nil {
        log.Fatal(err)
    }
    defer db.Close()

    // Execute a query
    rows, err := db.Query("SELECT 1 as num, 'Hello' as message")
    if err != nil {
        log.Fatal(err)
    }
    defer rows.Close()

    // Process results
    for rows.Next() {
        var num int
        var message string
        if err := rows.Scan(&num, &message); err != nil {
            log.Fatal(err)
        }
        log.Printf("num=%d, message=%s", num, message)
    }
}
```

## DSN Format

The driver supports two DSN formats:

### URL Style
```
user[:password]@account[/database[/schema]][?param1=value1&...]
```

Examples:
- `user@account`
- `user:password@account/mydb/myschema`
- `user@account?warehouse=wh&role=admin`

### Key-Value Style
```
account=<account>&user=<user>&password=<password>&...
```

Example:
```
account=myaccount&user=myuser&database=mydb&warehouse=wh
```

## Connection Parameters

| Parameter | Description |
|-----------|-------------|
| account | Snowflake account identifier (required) |
| user | Login user name (required) |
| password | Login password |
| database | Default database |
| schema | Default schema |
| warehouse | Default warehouse |
| role | Default role |
| authenticator | Authentication method (SNOWFLAKE, SNOWFLAKE_JWT, etc.) |
| private_key | PEM-encoded private key for JWT auth |
| private_key_file | Path to private key file |
| private_key_password | Password for encrypted private key |
| host | Snowflake host (derived from account if not specified) |
| port | Snowflake port (default: 443) |
| protocol | Connection protocol (default: https) |
| backend | Backend type: "wasm" (default) or "native" |

## JWT Authentication

For key-pair authentication:

```go
import "github.com/snowflakedb/universal-driver/go/snowflake"

cfg := &snowflake.Config{
    Account:            "myaccount",
    User:               "myuser",
    Host:               "myaccount.snowflakecomputing.com",
    Authenticator:      snowflake.AuthTypeJWT,
    PrivateKeyPEM:      privateKeyPEM,  // PEM string
    PrivateKeyPassword: "key-password", // If encrypted
    Database:           "mydb",
    Schema:             "myschema",
    Warehouse:          "mywh",
    Role:               "myrole",
}

drv := &snowflake.Driver{}
db := sql.OpenDB(snowflake.NewConnector(drv, cfg))
defer db.Close()
```

## Backend Selection

The driver supports two backends:

### WASM Backend (Default)
- Pure Go, no CGO required
- Portable across platforms
- Uses the WASM build of sf_core

### Native Backend (Optional)
- Requires CGO and libsf_core native library
- Better performance for large result sets
- FIPS compliance support

Set via DSN:
```
account=myaccount&user=myuser&backend=wasm
```

Or via Config:
```go
cfg.Backend = "wasm"  // or "native"
```

## Building

### Prerequisites

1. Go 1.24+
2. WASM module (for WASM backend):
   ```bash
   cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1
   ```

### Build

```bash
cd go
go build ./snowflake/...
```

### Test

```bash
# Unit tests only
go test -short ./snowflake/...

# Full tests (requires Snowflake connection)
go test -v ./snowflake/...
```

## Current Limitations

- **Chunk Downloading**: Large result sets (>16K rows) that span multiple chunks are partially supported. The first chunk is returned; subsequent chunks require chunk download implementation.
- **Parameter Binding**: Query parameter binding is not yet implemented.
- **Async Queries**: Asynchronous query execution is not yet implemented.
- **Native Backend**: The native CGO backend is not yet fully implemented.

## License

Apache 2.0
