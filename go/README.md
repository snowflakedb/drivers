# Go Snowflake Driver (WASM) - No CGO

This directory contains Go bindings for the Snowflake universal driver using WebAssembly with **standard WASI Preview 2 interfaces**.

## ✅ No CGO Required

```bash
CGO_ENABLED=0 go build ./...  # Works!
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Go Application                          │
├─────────────────────────────────────────────────────────────┤
│  wazero runtime (pure Go)                                   │
│    │                                                        │
│    ├── sf_core WASM module (1.9 MB)                         │
│    │     ├── Snowflake REST client                          │
│    │     ├── Retry logic & CRL handlers                     │
│    │     ├── Crypto (RustCrypto)                            │
│    │     └── imports wasi:sockets/tcp@0.2.4                 │
│    │                    │                                   │
│    └── WASI Preview 2 Shim (wasip2 package)                 │
│          ├── wasi:sockets/tcp@0.2.4  → Go net.Dial          │
│          ├── wasi:io/streams@0.2.4   → Go io.Read/Write     │
│          └── wasi:io/poll@0.2.4      → Go select            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                    Snowflake REST API
```

## Standard WASI Interfaces

The WASM module imports standard WASI Preview 2 interfaces:

| Interface | Purpose | Go Implementation |
|-----------|---------|-------------------|
| `wasi:sockets/tcp@0.2.4` | TCP connections | `net.Dial`, `net.Conn` |
| `wasi:io/streams@0.2.4` | I/O streams | `io.Reader`, `io.Writer` |
| `wasi:io/poll@0.2.4` | Async polling | Immediate ready |
| `wasi:clocks/monotonic-clock@0.2.4` | Timeouts | `time.Duration` |

**When wazero adds native WASI Preview 2 support, the `wasip2` shim can be removed.**

## Why WASI Sockets (not WASI HTTP)?

Using `wasi:sockets` instead of `wasi:http` allows the WASM module to:
- ✅ Implement custom **retry logic**
- ✅ Handle **CRL (Certificate Revocation List)** checking
- ✅ Control **connection pooling**
- ✅ Use **HTTP/1.1 keep-alive**

With `wasi:http`, these would be handled by the host, losing control.

## Quick Start

### 1. Build the WASM Module

```bash
cd /path/to/universal-driver
cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1
```

### 2. Run the Demo

```bash
cd go
go run ./cmd/demo
```

Output:
```
Loading WASM from: .../sf_core_wasm_reactor.wasm
WASM size: 1.91 MB
✓ WASI Preview 2 shim registered
✓ Module compiled
✓ Module instantiated
Driver version: 0.1.0

--- 1. database_new ---
✓ Got database handle

--- 2. database_init ---
✓ Database initialized

--- 3. connection_new ---
✓ Got connection handle

✓ Go WASM driver demo complete (no CGO!)

Using standard WASI Preview 2 interfaces:
  - wasi:sockets/tcp@0.2.4 (Go net.Dial)
  - wasi:io/streams@0.2.4 (Go io.Reader/Writer)

When wazero adds native support, the shim can be removed.
```

## Files

```
go/
├── go.mod
├── README.md
├── wasip2/
│   └── wasip2.go        # WASI Preview 2 shim
└── cmd/
    ├── demo/
    │   └── main.go      # Demo application
    └── check_imports/
        └── main.go      # Utility to list WASM imports
```

## License

Apache 2.0
