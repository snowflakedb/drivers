package snowflake

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"sync"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
	"github.com/snowflakedb/universal-driver/go/wasip2"
	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
	"google.golang.org/protobuf/proto"
)

const (
	resultOK             = 0
	resultError          = 1
	resultTransportError = 2
)

// WASMBackend implements Backend using a WASM module
type WASMBackend struct {
	runtime  wazero.Runtime
	module   api.Module
	compiled wazero.CompiledModule
	mu       sync.Mutex
}

// wasmMemory wraps wazero memory to implement Memory interface
type wasmMemory struct {
	mem api.Memory
}

func (m *wasmMemory) Read(offset, length uint32) ([]byte, bool) {
	return m.mem.Read(offset, length)
}

// NewWASMBackend creates a new WASM backend
func NewWASMBackend(ctx context.Context) (*WASMBackend, error) {
	wasmPath := findWasmFile()
	if wasmPath == "" {
		return nil, fmt.Errorf("WASM module not found")
	}

	wasmBytes, err := os.ReadFile(wasmPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read WASM module: %w", err)
	}

	r := wazero.NewRuntime(ctx)

	// WASI Preview 1
	wasi_snapshot_preview1.MustInstantiate(ctx, r)

	// WASI Preview 2 (sockets, streams, poll)
	if err := wasip2.Instantiate(ctx, r); err != nil {
		r.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASI Preview 2: %w", err)
	}

	compiled, err := r.CompileModule(ctx, wasmBytes)
	if err != nil {
		r.Close(ctx)
		return nil, fmt.Errorf("failed to compile WASM module: %w", err)
	}

	module, err := r.InstantiateModule(ctx, compiled, wazero.NewModuleConfig().
		WithSysWalltime().
		WithSysNanotime().
		WithStdout(os.Stdout).
		WithStderr(os.Stderr))
	if err != nil {
		compiled.Close(ctx)
		r.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}

	return &WASMBackend{
		runtime:  r,
		module:   module,
		compiled: compiled,
	}, nil
}

// findWasmFile searches for the WASM module file
func findWasmFile() string {
	// Try common locations
	candidates := []string{
		// Local file (for tests run from go/snowflake/)
		"sf_core_wasm_reactor.wasm",
		// From go/ directory
		"snowflake/sf_core_wasm_reactor.wasm",
		// From project root
		"target/wasm32-wasip1/release/sf_core_wasm_reactor.wasm",
		"go/snowflake/sf_core_wasm_reactor.wasm",
		// From go/snowflake or nested test directories
		"../target/wasm32-wasip1/release/sf_core_wasm_reactor.wasm",
		"../../target/wasm32-wasip1/release/sf_core_wasm_reactor.wasm",
	}

	// Also check relative to the executable
	if execPath, err := os.Executable(); err == nil {
		execDir := filepath.Dir(execPath)
		candidates = append(candidates,
			filepath.Join(execDir, "sf_core_wasm_reactor.wasm"),
			filepath.Join(execDir, "..", "target", "wasm32-wasip1", "release", "sf_core_wasm_reactor.wasm"),
		)
	}

	// Check WASM_PATH env var
	if envPath := os.Getenv("SNOWFLAKE_WASM_PATH"); envPath != "" {
		candidates = append([]string{envPath}, candidates...)
	}

	for _, path := range candidates {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}

	return ""
}

func (b *WASMBackend) Initialize(ctx context.Context) error {
	// Already initialized in NewWASMBackend
	return nil
}

func (b *WASMBackend) Close(ctx context.Context) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.module != nil {
		b.module.Close(ctx)
	}
	if b.compiled != nil {
		b.compiled.Close(ctx)
	}
	if b.runtime != nil {
		b.runtime.Close(ctx)
	}
	return nil
}

func (b *WASMBackend) GetMemory() Memory {
	if b.module != nil && b.module.Memory() != nil {
		return &wasmMemory{mem: b.module.Memory()}
	}
	return nil
}

// callProto calls a protobuf API method on the WASM module
func (b *WASMBackend) callProto(ctx context.Context, apiName, method string, req proto.Message, resp proto.Message) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	apiCallFn := b.module.ExportedFunction("api_call")
	getResultLen := b.module.ExportedFunction("get_result_len")
	getResult := b.module.ExportedFunction("get_result")
	clearResult := b.module.ExportedFunction("clear_result")
	allocBytes := b.module.ExportedFunction("alloc_bytes")
	deallocBytes := b.module.ExportedFunction("dealloc_bytes")

	apiPtr, apiLen := b.writeString(ctx, allocBytes, apiName)
	defer deallocBytes.Call(ctx, uint64(apiPtr), uint64(apiLen))

	methodPtr, methodLen := b.writeString(ctx, allocBytes, method)
	defer deallocBytes.Call(ctx, uint64(methodPtr), uint64(methodLen))

	var reqPtr uint32 = 0
	reqLen := uint32(0)
	if req != nil {
		reqBytes, err := proto.Marshal(req)
		if err != nil {
			return fmt.Errorf("failed to marshal request: %w", err)
		}
		reqLen = uint32(len(reqBytes))
		if reqLen > 0 {
			results, _ := allocBytes.Call(ctx, uint64(reqLen))
			reqPtr = uint32(results[0])
			b.module.Memory().Write(reqPtr, reqBytes)
			defer deallocBytes.Call(ctx, uint64(reqPtr), uint64(reqLen))
		}
	}

	results, err := apiCallFn.Call(ctx,
		uint64(apiPtr), uint64(apiLen),
		uint64(methodPtr), uint64(methodLen),
		uint64(reqPtr), uint64(reqLen))
	if err != nil {
		return fmt.Errorf("API call failed: %w", err)
	}

	resultCode := uint32(results[0])
	results, _ = getResultLen.Call(ctx)
	resultLen := uint32(results[0])

	var result []byte
	if resultLen > 0 {
		results, _ = allocBytes.Call(ctx, uint64(resultLen))
		resultPtr := uint32(results[0])
		getResult.Call(ctx, uint64(resultPtr))

		// Read immediately and copy to avoid memory aliasing
		rawResult, ok := b.module.Memory().Read(resultPtr, resultLen)
		if !ok {
			return fmt.Errorf("failed to read memory at 0x%x len %d", resultPtr, resultLen)
		}
		result = make([]byte, len(rawResult))
		copy(result, rawResult)

		deallocBytes.Call(ctx, uint64(resultPtr), uint64(resultLen))
	}
	clearResult.Call(ctx)

	switch resultCode {
	case resultOK:
		if resp != nil && len(result) > 0 {
			if err := proto.Unmarshal(result, resp); err != nil {
				return fmt.Errorf("failed to unmarshal response: %w", err)
			}
		}
		return nil
	case resultError:
		// Parse DriverException
		var exc pb.DriverException
		if err := proto.Unmarshal(result, &exc); err != nil {
			return fmt.Errorf("driver error (failed to parse): %v", result)
		}
		return &SnowflakeError{
			Code:    int(exc.GetStatusCode()),
			Message: exc.GetMessage(),
		}
	case resultTransportError:
		return fmt.Errorf("transport error: %s", string(result))
	default:
		return fmt.Errorf("unknown result code: %d", resultCode)
	}
}

func (b *WASMBackend) writeString(ctx context.Context, allocFn api.Function, s string) (uint32, uint32) {
	data := []byte(s)
	if len(data) == 0 {
		return 0, 0
	}
	results, _ := allocFn.Call(ctx, uint64(len(data)))
	ptr := uint32(results[0])
	b.module.Memory().Write(ptr, data)
	return ptr, uint32(len(data))
}

// DatabaseNew implements Backend
func (b *WASMBackend) DatabaseNew(ctx context.Context) (*pb.DatabaseHandle, error) {
	resp := &pb.DatabaseNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "database_new", &pb.DatabaseNewRequest{}, resp); err != nil {
		return nil, err
	}
	return resp.GetDbHandle(), nil
}

// DatabaseInit implements Backend
func (b *WASMBackend) DatabaseInit(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "database_init", &pb.DatabaseInitRequest{DbHandle: dbHandle}, &pb.DatabaseInitResponse{})
}

// DatabaseRelease implements Backend
func (b *WASMBackend) DatabaseRelease(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "database_release", &pb.DatabaseReleaseRequest{DbHandle: dbHandle}, &pb.DatabaseReleaseResponse{})
}

// ConnectionNew implements Backend
func (b *WASMBackend) ConnectionNew(ctx context.Context) (*pb.ConnectionHandle, error) {
	resp := &pb.ConnectionNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "connection_new", &pb.ConnectionNewRequest{}, resp); err != nil {
		return nil, err
	}
	return resp.GetConnHandle(), nil
}

// ConnectionSetOptionString implements Backend
func (b *WASMBackend) ConnectionSetOptionString(ctx context.Context, connHandle *pb.ConnectionHandle, key, value string) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_string",
		&pb.ConnectionSetOptionStringRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionStringResponse{})
}

// ConnectionSetOptionInt implements Backend
func (b *WASMBackend) ConnectionSetOptionInt(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value int64) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_int",
		&pb.ConnectionSetOptionIntRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionIntResponse{})
}

// ConnectionSetOptionDouble implements Backend
func (b *WASMBackend) ConnectionSetOptionDouble(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value float64) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_double",
		&pb.ConnectionSetOptionDoubleRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionDoubleResponse{})
}

// ConnectionSetOptionBytes implements Backend
func (b *WASMBackend) ConnectionSetOptionBytes(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value []byte) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_bytes",
		&pb.ConnectionSetOptionBytesRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionBytesResponse{})
}

// ConnectionInit implements Backend
func (b *WASMBackend) ConnectionInit(ctx context.Context, connHandle *pb.ConnectionHandle, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_init",
		&pb.ConnectionInitRequest{ConnHandle: connHandle, DbHandle: dbHandle},
		&pb.ConnectionInitResponse{})
}

// ConnectionRelease implements Backend
func (b *WASMBackend) ConnectionRelease(ctx context.Context, connHandle *pb.ConnectionHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_release",
		&pb.ConnectionReleaseRequest{ConnHandle: connHandle},
		&pb.ConnectionReleaseResponse{})
}

// StatementNew implements Backend
func (b *WASMBackend) StatementNew(ctx context.Context, connHandle *pb.ConnectionHandle) (*pb.StatementHandle, error) {
	resp := &pb.StatementNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "statement_new",
		&pb.StatementNewRequest{ConnHandle: connHandle}, resp); err != nil {
		return nil, err
	}
	return resp.GetStmtHandle(), nil
}

// StatementSetSqlQuery implements Backend
func (b *WASMBackend) StatementSetSqlQuery(ctx context.Context, stmtHandle *pb.StatementHandle, query string) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_set_sql_query",
		&pb.StatementSetSqlQueryRequest{StmtHandle: stmtHandle, Query: query},
		&pb.StatementSetSqlQueryResponse{})
}

// StatementSetOptionString implements Backend
func (b *WASMBackend) StatementSetOptionString(ctx context.Context, stmtHandle *pb.StatementHandle, key, value string) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_set_option_string",
		&pb.StatementSetOptionStringRequest{StmtHandle: stmtHandle, Key: key, Value: value},
		&pb.StatementSetOptionStringResponse{})
}

// StatementBindStream implements Backend
func (b *WASMBackend) StatementBindStream(ctx context.Context, stmtHandle *pb.StatementHandle, stream []byte) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_bind_stream",
		&pb.StatementBindStreamRequest{StmtHandle: stmtHandle, Stream: stream},
		&pb.StatementBindStreamResponse{})
}

// StatementExecuteQuery implements Backend
func (b *WASMBackend) StatementExecuteQuery(ctx context.Context, stmtHandle *pb.StatementHandle) (*pb.ExecuteResult, error) {
	resp := &pb.StatementExecuteQueryResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "statement_execute_query",
		&pb.StatementExecuteQueryRequest{StmtHandle: stmtHandle}, resp); err != nil {
		return nil, err
	}
	return resp.GetResult(), nil
}

// StatementRelease implements Backend
func (b *WASMBackend) StatementRelease(ctx context.Context, stmtHandle *pb.StatementHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_release",
		&pb.StatementReleaseRequest{StmtHandle: stmtHandle},
		&pb.StatementReleaseResponse{})
}

// ReleaseArrowResult implements Backend
func (b *WASMBackend) ReleaseArrowResult(ctx context.Context, handle uint64) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	releaseFn := b.module.ExportedFunction("release_arrow_result")
	if releaseFn == nil {
		return nil // Function not available, ignore
	}
	_, err := releaseFn.Call(ctx, handle)
	return err
}
