// Package main demonstrates using the Snowflake driver WASM reactor from Go.
//
// This demo uses wazero with a WASI Preview 2 shim for sockets.
// Go provides the WASI interfaces, allowing sf_core to use its native
// retry logic and CRL handlers.
//
// Build the WASM reactor first:
//
//	cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1
//
// Then run this demo:
//
//	go run ./cmd/demo
package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/snowflakedb/universal-driver/go/wasip2"
	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

func main() {
	ctx := context.Background()

	wasmPath := findWasmFile()
	if wasmPath == "" {
		log.Fatal("Could not find sf_core_wasm_reactor.wasm")
	}

	fmt.Printf("Loading WASM from: %s\n", wasmPath)

	wasmBytes, err := os.ReadFile(wasmPath)
	if err != nil {
		log.Fatalf("Failed to read WASM: %v", err)
	}

	fmt.Printf("WASM size: %.2f MB\n", float64(len(wasmBytes))/(1024*1024))

	// Create runtime
	r := wazero.NewRuntime(ctx)
	defer r.Close(ctx)

	// Instantiate WASI Preview 1 (for basic file I/O, etc.)
	wasi_snapshot_preview1.MustInstantiate(ctx, r)

	// Instantiate WASI Preview 2 shim (for sockets)
	_, err = wasip2.Instantiate(ctx, r)
	if err != nil {
		log.Fatalf("Failed to instantiate WASI Preview 2: %v", err)
	}
	fmt.Printf("✓ WASI Preview 2 shim registered\n")

	// Compile
	compiled, err := r.CompileModule(ctx, wasmBytes)
	if err != nil {
		log.Fatalf("Failed to compile: %v", err)
	}
	defer compiled.Close(ctx)

	fmt.Printf("✓ Module compiled\n")

	// Instantiate
	mod, err := r.InstantiateModule(ctx, compiled, wazero.NewModuleConfig().
		WithStdout(os.Stdout).
		WithStderr(os.Stderr))
	if err != nil {
		log.Fatalf("Failed to instantiate: %v", err)
	}
	defer mod.Close(ctx)

	fmt.Printf("✓ Module instantiated\n")

	// Get version
	version := getVersion(ctx, mod)
	fmt.Printf("Driver version: %s\n", version)

	// Demo: API flow
	fmt.Printf("\n--- 1. database_new ---\n")
	code, result := apiCall(ctx, mod, "DatabaseDriver", "database_new", nil)
	if code != 0 {
		printError(code, result)
		return
	}
	dbHandle := result
	fmt.Printf("✓ Got database handle\n")

	fmt.Printf("\n--- 2. database_init ---\n")
	code, result = apiCall(ctx, mod, "DatabaseDriver", "database_init", dbHandle)
	if code != 0 {
		printError(code, result)
		return
	}
	fmt.Printf("✓ Database initialized\n")

	fmt.Printf("\n--- 3. connection_new ---\n")
	code, result = apiCall(ctx, mod, "DatabaseDriver", "connection_new", dbHandle)
	if code != 0 {
		printError(code, result)
		return
	}
	_ = result // connHandle
	fmt.Printf("✓ Got connection handle\n")

	fmt.Printf("\n✓ Go WASM driver demo complete (no CGO!)\n")
	fmt.Printf("\nUsing standard WASI Preview 2 interfaces:\n")
	fmt.Printf("  - wasi:sockets/tcp@0.2.4 (Go net.Dial)\n")
	fmt.Printf("  - wasi:io/streams@0.2.4 (Go io.Reader/Writer)\n")
	fmt.Printf("\nWhen wazero adds native support, the shim can be removed.\n")
}

func printError(code uint32, result []byte) {
	if code == 2 {
		fmt.Printf("Transport error: %s\n", string(result))
	} else if code == 1 {
		fmt.Printf("App error: %s\n", string(result))
	}
}

func getVersion(ctx context.Context, mod api.Module) string {
	getVersionLen := mod.ExportedFunction("get_version_len")
	getVersion := mod.ExportedFunction("get_version")
	allocBytes := mod.ExportedFunction("alloc_bytes")
	deallocBytes := mod.ExportedFunction("dealloc_bytes")

	results, _ := getVersionLen.Call(ctx)
	versionLen := uint32(results[0])

	results, _ = allocBytes.Call(ctx, uint64(versionLen))
	bufPtr := uint32(results[0])
	defer deallocBytes.Call(ctx, uint64(bufPtr), uint64(versionLen))

	getVersion.Call(ctx, uint64(bufPtr))
	data, _ := mod.Memory().Read(bufPtr, versionLen)
	return string(data)
}

func apiCall(ctx context.Context, mod api.Module, apiName, method string, request []byte) (uint32, []byte) {
	apiCallFn := mod.ExportedFunction("api_call")
	getResultLen := mod.ExportedFunction("get_result_len")
	getResult := mod.ExportedFunction("get_result")
	clearResult := mod.ExportedFunction("clear_result")
	allocBytes := mod.ExportedFunction("alloc_bytes")
	deallocBytes := mod.ExportedFunction("dealloc_bytes")

	apiPtr, apiLen := writeString(ctx, mod, allocBytes, apiName)
	defer deallocBytes.Call(ctx, uint64(apiPtr), uint64(apiLen))

	methodPtr, methodLen := writeString(ctx, mod, allocBytes, method)
	defer deallocBytes.Call(ctx, uint64(methodPtr), uint64(methodLen))

	var reqPtr uint32 = 0
	reqLen := uint32(len(request))
	if reqLen > 0 {
		results, _ := allocBytes.Call(ctx, uint64(reqLen))
		reqPtr = uint32(results[0])
		mod.Memory().Write(reqPtr, request)
		defer deallocBytes.Call(ctx, uint64(reqPtr), uint64(reqLen))
	}

	results, _ := apiCallFn.Call(ctx,
		uint64(apiPtr), uint64(apiLen),
		uint64(methodPtr), uint64(methodLen),
		uint64(reqPtr), uint64(reqLen))
	code := uint32(results[0])

	results, _ = getResultLen.Call(ctx)
	resultLen := uint32(results[0])

	var result []byte
	if resultLen > 0 {
		results, _ = allocBytes.Call(ctx, uint64(resultLen))
		resultPtr := uint32(results[0])
		getResult.Call(ctx, uint64(resultPtr))
		result, _ = mod.Memory().Read(resultPtr, resultLen)
		deallocBytes.Call(ctx, uint64(resultPtr), uint64(resultLen))
	}

	clearResult.Call(ctx)
	return code, result
}

func writeString(ctx context.Context, mod api.Module, allocBytes api.Function, s string) (uint32, uint32) {
	if len(s) == 0 {
		return 0, 0
	}
	results, _ := allocBytes.Call(ctx, uint64(len(s)))
	ptr := uint32(results[0])
	mod.Memory().Write(ptr, []byte(s))
	return ptr, uint32(len(s))
}

func findWasmFile() string {
	candidates := []string{
		"../target/wasm32-wasip1/release/sf_core_wasm_reactor.wasm",
		"../../target/wasm32-wasip1/release/sf_core_wasm_reactor.wasm",
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			abs, _ := filepath.Abs(c)
			return abs
		}
	}
	return ""
}
