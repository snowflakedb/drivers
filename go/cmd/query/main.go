// Package main demonstrates running a real query through the WASM driver.
//
// Usage:
//
//	go run ./cmd/query
//
// Requirements:
//   - VPN connection to Snowflake
//   - WASM module built: cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1
//   - Credentials in /Users/snoonan/parameters.json
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/apache/arrow-go/v18/arrow/ipc"
	pb "github.com/snowflakedb/universal-driver/go/protobuf"
	"github.com/snowflakedb/universal-driver/go/wasip2"
	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
	"google.golang.org/protobuf/proto"
)

var (
	mod api.Module
)

func main() {
	ctx := context.Background()

	// Load credentials
	creds, err := loadCredentials("/Users/snoonan/parameters.json")
	if err != nil {
		log.Fatalf("Failed to load credentials: %v", err)
	}
	fmt.Printf("Account: %s\n", creds.Account)
	fmt.Printf("User: %s\n", creds.User)
	fmt.Printf("Host: %s\n", creds.Host)

	// Load WASM
	wasmPath := findWasmFile()
	if wasmPath == "" {
		log.Fatal("WASM file not found. Run: cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1")
	}

	wasmBytes, err := os.ReadFile(wasmPath)
	if err != nil {
		log.Fatalf("Failed to read WASM: %v", err)
	}
	fmt.Printf("WASM loaded: %.2f MB\n", float64(len(wasmBytes))/(1024*1024))

	// Create runtime
	r := wazero.NewRuntime(ctx)
	defer r.Close(ctx)

	// WASI Preview 1 (basic filesystem, clock, etc.)
	wasi_snapshot_preview1.MustInstantiate(ctx, r)

	// WASI Preview 2 (sockets, streams, poll)
	wasip2.Debug = true // Enable for debugging TLS
	err = wasip2.Instantiate(ctx, r)
	if err != nil {
		log.Fatalf("Failed to instantiate WASI Preview 2: %v", err)
	}

	// Compile and instantiate
	compiled, err := r.CompileModule(ctx, wasmBytes)
	if err != nil {
		log.Fatalf("Failed to compile: %v", err)
	}
	defer compiled.Close(ctx)

	// Configure module with real wall time
	mod, err = r.InstantiateModule(ctx, compiled, wazero.NewModuleConfig().
		WithSysWalltime().  // Enable real wall clock
		WithSysNanotime(). // Enable real monotonic clock
		WithStdout(os.Stdout).
		WithStderr(os.Stderr))
	if err != nil {
		log.Fatalf("Failed to instantiate: %v", err)
	}
	defer mod.Close(ctx)

	fmt.Printf("Driver version: %s\n\n", getVersion(ctx))

	// === DRIVER FLOW ===

	// 1. Create database
	fmt.Println("1. Creating database...")
	dbHandle := databaseNew(ctx)

	// 2. Init database
	fmt.Println("2. Initializing database...")
	databaseInit(ctx, dbHandle)

	// 3. Create connection
	fmt.Println("3. Creating connection...")
	connHandle := connectionNew(ctx, dbHandle)

	// 4. Set connection options
	fmt.Println("4. Setting connection options...")
	setOption(ctx, connHandle, "host", creds.Host)
	setOption(ctx, connHandle, "account", creds.Account)
	setOption(ctx, connHandle, "user", creds.User)
	setOption(ctx, connHandle, "authenticator", "SNOWFLAKE_JWT")
	setOption(ctx, connHandle, "private_key", creds.PrivateKey)
	setOption(ctx, connHandle, "private_key_password", creds.PrivateKeyPassword)
	setOption(ctx, connHandle, "database", creds.Database)
	setOption(ctx, connHandle, "schema", creds.Schema)
	setOption(ctx, connHandle, "warehouse", creds.Warehouse)
	setOption(ctx, connHandle, "role", creds.Role)

	// 5. Init connection (performs login)
	fmt.Println("5. Connecting to Snowflake...")
	err = connectionInit(ctx, connHandle, dbHandle)
	if err != nil {
		log.Fatalf("Connection failed: %v", err)
	}
	fmt.Println("   ✓ Connected!")

	// 6. Create statement
	fmt.Println("6. Creating statement...")
	stmtHandle := statementNew(ctx, connHandle)

	// 7. Set query
	query := "SELECT 1 as num, 'Hello from Go WASM!' as message"
	fmt.Printf("7. Setting query: %s\n", query)
	statementSetQuery(ctx, stmtHandle, query)

	// 8. Execute query
	fmt.Println("8. Executing query...")
	err = statementExecute(ctx, stmtHandle)
	if err != nil {
		log.Fatalf("Query failed: %v", err)
	}

	fmt.Println("\n✅ Query executed successfully via Go WASM (no CGO)!")
}

type Credentials struct {
	User               string
	Password           string
	Account            string
	Role               string
	Warehouse          string
	Database           string
	Schema             string
	Host               string
	PrivateKey         string
	PrivateKeyPassword string
}

func loadCredentials(path string) (*Credentials, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var params struct {
		TestConnection struct {
			User               string   `json:"SNOWFLAKE_TEST_USER"`
			Password           string   `json:"SNOWFLAKE_TEST_PASSWORD"`
			Account            string   `json:"SNOWFLAKE_TEST_ACCOUNT"`
			Role               string   `json:"SNOWFLAKE_TEST_ROLE"`
			Warehouse          string   `json:"SNOWFLAKE_TEST_WAREHOUSE"`
			Database           string   `json:"SNOWFLAKE_TEST_DATABASE"`
			Schema             string   `json:"SNOWFLAKE_TEST_SCHEMA"`
			Host               string   `json:"SNOWFLAKE_TEST_HOST"`
			PrivateKeyPassword string   `json:"SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD"`
			PrivateKeyContents []string `json:"SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS"`
		} `json:"testconnection"`
	}

	if err := json.Unmarshal(data, &params); err != nil {
		return nil, err
	}

	return &Credentials{
		User:               params.TestConnection.User,
		Password:           params.TestConnection.Password,
		Account:            params.TestConnection.Account,
		Role:               params.TestConnection.Role,
		Warehouse:          params.TestConnection.Warehouse,
		Database:           params.TestConnection.Database,
		Schema:             params.TestConnection.Schema,
		Host:               params.TestConnection.Host,
		PrivateKey:         strings.Join(params.TestConnection.PrivateKeyContents, "\n"),
		PrivateKeyPassword: params.TestConnection.PrivateKeyPassword,
	}, nil
}

func databaseNew(ctx context.Context) *pb.DatabaseHandle {
	req := &pb.DatabaseNewRequest{}
	resp := &pb.DatabaseNewResponse{}
	callProto(ctx, "database_new", req, resp)
	return resp.DbHandle
}

func databaseInit(ctx context.Context, dbHandle *pb.DatabaseHandle) {
	req := &pb.DatabaseInitRequest{DbHandle: dbHandle}
	resp := &pb.DatabaseInitResponse{}
	callProto(ctx, "database_init", req, resp)
}

func connectionNew(ctx context.Context, dbHandle *pb.DatabaseHandle) *pb.ConnectionHandle {
	req := &pb.ConnectionNewRequest{}
	resp := &pb.ConnectionNewResponse{}
	callProto(ctx, "connection_new", req, resp)
	return resp.ConnHandle
}

func setOption(ctx context.Context, connHandle *pb.ConnectionHandle, key, value string) {
	req := &pb.ConnectionSetOptionStringRequest{
		ConnHandle: connHandle,
		Key:        key,
		Value:      value,
	}
	resp := &pb.ConnectionSetOptionStringResponse{}
	callProto(ctx, "connection_set_option_string", req, resp)
}

func connectionInit(ctx context.Context, connHandle *pb.ConnectionHandle, dbHandle *pb.DatabaseHandle) error {
	req := &pb.ConnectionInitRequest{
		ConnHandle: connHandle,
		DbHandle:   dbHandle,
	}
	resp := &pb.ConnectionInitResponse{}
	return callProtoWithError(ctx, "connection_init", req, resp)
}

func statementNew(ctx context.Context, connHandle *pb.ConnectionHandle) *pb.StatementHandle {
	req := &pb.StatementNewRequest{ConnHandle: connHandle}
	resp := &pb.StatementNewResponse{}
	callProto(ctx, "statement_new", req, resp)
	return resp.StmtHandle
}

func statementSetQuery(ctx context.Context, stmtHandle *pb.StatementHandle, query string) {
	req := &pb.StatementSetSqlQueryRequest{
		StmtHandle: stmtHandle,
		Query:      query,
	}
	resp := &pb.StatementSetSqlQueryResponse{}
	callProto(ctx, "statement_set_sql_query", req, resp)
}

func statementExecute(ctx context.Context, stmtHandle *pb.StatementHandle) error {
	req := &pb.StatementExecuteQueryRequest{StmtHandle: stmtHandle}
	resp := &pb.StatementExecuteQueryResponse{}
	err := callProtoWithError(ctx, "statement_execute_query", req, resp)
	if err != nil {
		return err
	}

	if resp.Result == nil {
		return fmt.Errorf("no result in response")
	}

	// Handle WASM zero-copy Arrow result
	if wasmResult := resp.Result.WasmResult; wasmResult != nil {
		fmt.Printf("\n   Zero-copy WASM Arrow result:\n")
		fmt.Printf("   Total rows: %d\n", wasmResult.TotalRows)
		fmt.Printf("   Batches: %d\n", len(wasmResult.Batches))
		fmt.Printf("   Release handle: %d\n", wasmResult.ReleaseHandle)
		
		// Parse schema from IPC
		if len(wasmResult.SchemaIpc) > 0 {
			reader, err := ipc.NewReader(bytes.NewReader(wasmResult.SchemaIpc))
			if err != nil {
				return fmt.Errorf("failed to parse schema IPC: %w", err)
			}
			schema := reader.Schema()
			reader.Release()
			
			fmt.Printf("   Schema: %d fields\n", schema.NumFields())
			for i := 0; i < schema.NumFields(); i++ {
				field := schema.Field(i)
				fmt.Printf("     - %s: %s\n", field.Name, field.Type)
			}
		}
		
		// Read data directly from WASM memory (zero-copy!)
		for batchIdx, batch := range wasmResult.Batches {
			fmt.Printf("   Batch %d: %d rows, %d columns\n", batchIdx, batch.NumRows, len(batch.Columns))
			
			for colIdx, col := range batch.Columns {
				fmt.Printf("     Column %d buffers:\n", colIdx)
				
				// Read validity buffer directly from WASM memory
				if col.Validity != nil && col.Validity.Length > 0 {
					validityData, ok := mod.Memory().Read(col.Validity.Offset, col.Validity.Length)
					if ok {
						fmt.Printf("       Validity: %d bytes @ offset 0x%x (zero-copy view)\n", 
							len(validityData), col.Validity.Offset)
					}
				}
				
				// Read data buffers directly from WASM memory
				for bufIdx, dataBuf := range col.Data {
					if dataBuf.Length > 0 {
						data, ok := mod.Memory().Read(dataBuf.Offset, dataBuf.Length)
						if ok {
							fmt.Printf("       Data[%d]: %d bytes @ offset 0x%x (zero-copy view)\n", 
								bufIdx, len(data), dataBuf.Offset)
							// Print first few bytes as preview
							preview := data
							if len(preview) > 32 {
								preview = preview[:32]
							}
							fmt.Printf("         Preview: %v...\n", preview)
						}
					}
				}
				
				// Read offsets buffer (for strings)
				if col.Offsets != nil && col.Offsets.Length > 0 {
					offsetsData, ok := mod.Memory().Read(col.Offsets.Offset, col.Offsets.Length)
					if ok {
						fmt.Printf("       Offsets: %d bytes @ offset 0x%x (zero-copy view)\n", 
							len(offsetsData), col.Offsets.Offset)
					}
				}
			}
		}
		
		// Release the Arrow data in WASM memory
		releaseArrowResult(ctx, wasmResult.ReleaseHandle)
		fmt.Println("   ✓ Released WASM memory")
		
		fmt.Printf("   Rows affected: %d\n", resp.Result.RowsAffected)
		return nil
	}

	// Fallback: IPC serialized result (shouldn't happen with WASM)
	if resp.Result.Stream != nil && len(resp.Result.Stream.Value) > 0 {
		ipcBytes := resp.Result.Stream.Value
		fmt.Printf("\n   Arrow IPC response: %d bytes (serialized, not zero-copy)\n", len(ipcBytes))
		
		reader, err := ipc.NewReader(bytes.NewReader(ipcBytes))
		if err != nil {
			return fmt.Errorf("failed to create Arrow IPC reader: %w", err)
		}
		defer reader.Release()

		schema := reader.Schema()
		fmt.Printf("   Schema: %d fields\n", schema.NumFields())
		
		totalRows := int64(0)
		for reader.Next() {
			rec := reader.Record()
			totalRows += rec.NumRows()
		}
		fmt.Printf("   Total rows: %d\n", totalRows)
	}

	return nil
}

// releaseArrowResult calls the WASM export to free Arrow memory
func releaseArrowResult(ctx context.Context, handle uint64) {
	releaseFn := mod.ExportedFunction("release_arrow_result")
	if releaseFn != nil {
		releaseFn.Call(ctx, handle)
	}
}

func callProto(ctx context.Context, method string, req, resp proto.Message) {
	if err := callProtoWithError(ctx, method, req, resp); err != nil {
		log.Fatalf("%s failed: %v", method, err)
	}
}

func callProtoWithError(ctx context.Context, method string, req, resp proto.Message) error {
	reqBytes, _ := proto.Marshal(req)

	code, result := apiCall(ctx, "DatabaseDriver", method, reqBytes)
	if code != 0 {
		// Try to decode as DriverException
		var exc pb.DriverException
		if err := proto.Unmarshal(result, &exc); err == nil {
			// Check for specific error types
			if exc.Error != nil {
				if mp := exc.Error.GetMissingParameter(); mp != nil {
					return fmt.Errorf("%s: missing parameter '%s' (code: %v)", method, mp.Parameter, exc.StatusCode)
				}
				if iv := exc.Error.GetInvalidParameterValue(); iv != nil {
					return fmt.Errorf("%s: invalid parameter '%s'='%s': %s (code: %v)", method, iv.Parameter, iv.Value, iv.GetExplanation(), exc.StatusCode)
				}
				if ae := exc.Error.GetAuthError(); ae != nil {
					return fmt.Errorf("%s: auth error: %s (code: %v)", method, ae.Detail, exc.StatusCode)
				}
				if le := exc.Error.GetLoginError(); le != nil {
					return fmt.Errorf("%s: login error [%d]: %s (code: %v)", method, le.Code, le.Message, exc.StatusCode)
				}
			}
			return fmt.Errorf("%s: %s (code: %v, report: %s)", method, exc.Message, exc.StatusCode, exc.Report)
		}
		return fmt.Errorf("%s failed with code %d: %s", method, code, string(result))
	}

	return proto.Unmarshal(result, resp)
}

func getVersion(ctx context.Context) string {
	getVersionLen := mod.ExportedFunction("get_version_len")
	getVersionFn := mod.ExportedFunction("get_version")
	allocBytes := mod.ExportedFunction("alloc_bytes")
	deallocBytes := mod.ExportedFunction("dealloc_bytes")

	results, _ := getVersionLen.Call(ctx)
	versionLen := uint32(results[0])

	results, _ = allocBytes.Call(ctx, uint64(versionLen))
	bufPtr := uint32(results[0])
	defer deallocBytes.Call(ctx, uint64(bufPtr), uint64(versionLen))

	getVersionFn.Call(ctx, uint64(bufPtr))
	data, _ := mod.Memory().Read(bufPtr, versionLen)
	return string(data)
}

func apiCall(ctx context.Context, apiName, method string, request []byte) (uint32, []byte) {
	apiCallFn := mod.ExportedFunction("api_call")
	getResultLen := mod.ExportedFunction("get_result_len")
	getResult := mod.ExportedFunction("get_result")
	clearResult := mod.ExportedFunction("clear_result")
	allocBytes := mod.ExportedFunction("alloc_bytes")
	deallocBytes := mod.ExportedFunction("dealloc_bytes")

	apiPtr, apiLen := writeString(ctx, allocBytes, apiName)
	defer deallocBytes.Call(ctx, uint64(apiPtr), uint64(apiLen))

	methodPtr, methodLen := writeString(ctx, allocBytes, method)
	defer deallocBytes.Call(ctx, uint64(methodPtr), uint64(methodLen))

	var reqPtr uint32 = 0
	reqLen := uint32(len(request))
	if reqLen > 0 {
		results, _ := allocBytes.Call(ctx, uint64(reqLen))
		reqPtr = uint32(results[0])
		mod.Memory().Write(reqPtr, request)
		defer deallocBytes.Call(ctx, uint64(reqPtr), uint64(reqLen))
	}

	results, err := apiCallFn.Call(ctx,
		uint64(apiPtr), uint64(apiLen),
		uint64(methodPtr), uint64(methodLen),
		uint64(reqPtr), uint64(reqLen))
	if err != nil {
		log.Printf("api_call failed: %v", err)
		return 1, nil
	}
	if len(results) == 0 {
		log.Printf("api_call returned no results")
		return 1, nil
	}
	code := uint32(results[0])

	results, _ = getResultLen.Call(ctx)
	resultLen := uint32(results[0])

	var result []byte
	if resultLen > 0 {
		results, _ = allocBytes.Call(ctx, uint64(resultLen))
		resultPtr := uint32(results[0])
		
		getResult.Call(ctx, uint64(resultPtr))
		
		// Read the result immediately and make a copy.
		// Note: wazero's Memory.Read() returns a slice that may be invalidated
		// by subsequent WASM calls, so we must copy immediately.
		rawResult, ok := mod.Memory().Read(resultPtr, resultLen)
		if !ok {
			log.Printf("Failed to read memory at 0x%x len %d", resultPtr, resultLen)
			return 1, nil
		}
		result = make([]byte, len(rawResult))
		copy(result, rawResult)
		
		deallocBytes.Call(ctx, uint64(resultPtr), uint64(resultLen))
	}

	clearResult.Call(ctx)
	return code, result
}

func writeString(ctx context.Context, allocBytes api.Function, s string) (uint32, uint32) {
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

// Suppress unused import warning
var _ = io.Discard
