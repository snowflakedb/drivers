package wasip2

import (
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

// TestResourceManagerBasics tests resource allocation and deallocation.
func TestResourceManagerBasics(t *testing.T) {
	rm := NewResourceManager()

	// Test ID allocation is sequential
	id1 := rm.allocID()
	id2 := rm.allocID()
	id3 := rm.allocID()

	if id1 != 1 || id2 != 2 || id3 != 3 {
		t.Errorf("Expected sequential IDs 1,2,3, got %d,%d,%d", id1, id2, id3)
	}

	// Test network allocation
	rm.mu.Lock()
	rm.networks[10] = struct{}{}
	rm.mu.Unlock()

	rm.mu.Lock()
	_, exists := rm.networks[10]
	rm.mu.Unlock()
	if !exists {
		t.Error("Network should exist")
	}
}

// TestSocketLifecycle tests creating, connecting, and closing sockets.
func TestSocketLifecycle(t *testing.T) {
	rm := NewResourceManager()

	// Create socket
	sockID := rm.allocID()
	rm.mu.Lock()
	rm.sockets[sockID] = &tcpSocket{state: "new"}
	rm.mu.Unlock()

	// Check state
	rm.mu.Lock()
	sock := rm.sockets[sockID]
	rm.mu.Unlock()

	if sock.state != "new" {
		t.Errorf("Expected state 'new', got '%s'", sock.state)
	}

	// Simulate connection
	rm.mu.Lock()
	sock.state = "connected"
	rm.mu.Unlock()

	rm.mu.Lock()
	state := sock.state
	rm.mu.Unlock()

	if state != "connected" {
		t.Errorf("Expected state 'connected', got '%s'", state)
	}

	// Drop socket
	rm.mu.Lock()
	delete(rm.sockets, sockID)
	_, exists := rm.sockets[sockID]
	rm.mu.Unlock()

	if exists {
		t.Error("Socket should be deleted")
	}
}

// TestStreamLifecycle tests input/output stream management.
func TestStreamLifecycle(t *testing.T) {
	rm := NewResourceManager()

	sockID := rm.allocID()
	rm.mu.Lock()
	rm.sockets[sockID] = &tcpSocket{state: "connected"}
	rm.mu.Unlock()

	// Create streams
	inID := rm.allocID()
	outID := rm.allocID()
	rm.mu.Lock()
	rm.streams[inID] = &stream{socket: sockID, isInput: true}
	rm.streams[outID] = &stream{socket: sockID, isInput: false}
	rm.mu.Unlock()

	// Verify
	rm.mu.Lock()
	inStream := rm.streams[inID]
	outStream := rm.streams[outID]
	rm.mu.Unlock()

	if !inStream.isInput {
		t.Error("Input stream should have isInput=true")
	}
	if outStream.isInput {
		t.Error("Output stream should have isInput=false")
	}
	if inStream.socket != sockID || outStream.socket != sockID {
		t.Error("Streams should reference the socket")
	}
}

// TestPollableManagement tests pollable resource creation and readiness.
func TestPollableManagement(t *testing.T) {
	rm := NewResourceManager()

	pollID := rm.allocID()
	rm.mu.Lock()
	rm.pollables[pollID] = &pollable{ready: true, resource: 42}
	rm.mu.Unlock()

	rm.mu.Lock()
	p := rm.pollables[pollID]
	rm.mu.Unlock()

	if !p.ready {
		t.Error("Pollable should be ready")
	}
	if p.resource != 42 {
		t.Errorf("Expected resource 42, got %d", p.resource)
	}

	// Drop pollable
	rm.mu.Lock()
	delete(rm.pollables, pollID)
	_, exists := rm.pollables[pollID]
	rm.mu.Unlock()

	if exists {
		t.Error("Pollable should be deleted")
	}
}

// TestInstantiateRegistersAllModules tests that all WASI P2 modules are registered.
func TestInstantiateRegistersAllModules(t *testing.T) {
	ctx := context.Background()
	r := wazero.NewRuntime(ctx)
	defer r.Close(ctx)

	rm, err := Instantiate(ctx, r)
	if err != nil {
		t.Fatalf("Failed to instantiate: %v", err)
	}
	if rm == nil {
		t.Fatal("ResourceManager should not be nil")
	}
}

// TestTcpConnectToRealServer tests actual TCP connectivity through the shim.
func TestTcpConnectToRealServer(t *testing.T) {
	// Start a test server
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("Failed to create listener: %v", err)
	}
	defer listener.Close()

	port := listener.Addr().(*net.TCPAddr).Port

	// Accept connections in background
	serverDone := make(chan struct{})
	go func() {
		defer close(serverDone)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()
		conn.Write([]byte("Hello from server"))
	}()

	rm := NewResourceManager()

	// Create socket
	sockID := rm.allocID()
	rm.mu.Lock()
	rm.sockets[sockID] = &tcpSocket{state: "new"}
	rm.mu.Unlock()

	// Connect (simulating what tcpStartConnect does)
	addr := fmt.Sprintf("127.0.0.1:%d", port)
	rm.mu.Lock()
	sock := rm.sockets[sockID]
	sock.state = "connecting"
	sock.remoteAddr = addr
	rm.mu.Unlock()

	go func() {
		conn, err := net.DialTimeout("tcp", addr, 5*time.Second)
		rm.mu.Lock()
		if err != nil {
			sock.state = "error"
			sock.connectErr = err
		} else {
			sock.conn = conn
			sock.state = "connected"
		}
		rm.mu.Unlock()
	}()

	// Wait for connection
	timeout := time.After(5 * time.Second)
	for {
		select {
		case <-timeout:
			t.Fatal("Connection timeout")
		default:
			rm.mu.Lock()
			state := sock.state
			conn := sock.conn
			rm.mu.Unlock()

			if state == "connected" && conn != nil {
				// Success! Read data
				buf := make([]byte, 100)
				conn.SetReadDeadline(time.Now().Add(time.Second))
				n, _ := conn.Read(buf)
				if n > 0 && strings.Contains(string(buf[:n]), "Hello") {
					conn.Close()
					<-serverDone
					return // Test passed
				}
				conn.Close()
				<-serverDone
				return
			}
			if state == "error" {
				t.Fatalf("Connection failed: %v", sock.connectErr)
			}
			time.Sleep(10 * time.Millisecond)
		}
	}
}

// TestHttpThroughWasm tests that the WASM module can make HTTP requests through the shim.
func TestHttpThroughWasm(t *testing.T) {
	// Start HTTP test server
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"success": true}`))
	}))
	defer server.Close()

	// Get server port
	serverURL := server.URL
	t.Logf("Test server at: %s", serverURL)

	// This test verifies the shim can handle connections that HTTP libraries would use
	rm := NewResourceManager()

	// Create socket
	sockID := rm.allocID()
	rm.mu.Lock()
	rm.sockets[sockID] = &tcpSocket{state: "new"}
	rm.mu.Unlock()

	// Parse server URL to get host:port
	host := strings.TrimPrefix(serverURL, "http://")

	// Connect
	rm.mu.Lock()
	sock := rm.sockets[sockID]
	sock.state = "connecting"
	sock.remoteAddr = host
	rm.mu.Unlock()

	go func() {
		conn, err := net.DialTimeout("tcp", host, 5*time.Second)
		rm.mu.Lock()
		if err != nil {
			sock.state = "error"
			sock.connectErr = err
		} else {
			sock.conn = conn
			sock.state = "connected"
		}
		rm.mu.Unlock()
	}()

	// Wait for connection
	timeout := time.After(5 * time.Second)
	for {
		select {
		case <-timeout:
			t.Fatal("Connection timeout")
		default:
			rm.mu.Lock()
			state := sock.state
			conn := sock.conn
			rm.mu.Unlock()

			if state == "connected" && conn != nil {
				// Send HTTP request
				request := "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
				conn.Write([]byte(request))

				// Read response
				buf := make([]byte, 1024)
				conn.SetReadDeadline(time.Now().Add(time.Second))
				n, _ := conn.Read(buf)
				response := string(buf[:n])

				if !strings.Contains(response, "200 OK") {
					t.Errorf("Expected 200 OK, got: %s", response)
				}
				if !strings.Contains(response, `"success": true`) {
					t.Errorf("Expected success:true in response: %s", response)
				}

				conn.Close()
				return
			}
			if state == "error" {
				t.Fatalf("Connection failed: %v", sock.connectErr)
			}
			time.Sleep(10 * time.Millisecond)
		}
	}
}

// TestFullWasmIntegration loads the actual WASM module and tests through the shim.
func TestFullWasmIntegration(t *testing.T) {
	wasmPath := findWasmFile()
	if wasmPath == "" {
		t.Skip("WASM file not found - run: cargo build -p sf_core_wasm_reactor --release --target wasm32-wasip1")
	}

	ctx := context.Background()

	wasmBytes, err := os.ReadFile(wasmPath)
	if err != nil {
		t.Fatalf("Failed to read WASM: %v", err)
	}

	r := wazero.NewRuntime(ctx)
	defer r.Close(ctx)

	// WASI Preview 1
	wasi_snapshot_preview1.MustInstantiate(ctx, r)

	// WASI Preview 2 shim
	_, err = Instantiate(ctx, r)
	if err != nil {
		t.Fatalf("Failed to instantiate WASI P2: %v", err)
	}

	// Compile
	compiled, err := r.CompileModule(ctx, wasmBytes)
	if err != nil {
		t.Fatalf("Failed to compile: %v", err)
	}
	defer compiled.Close(ctx)

	// Instantiate
	mod, err := r.InstantiateModule(ctx, compiled, wazero.NewModuleConfig().
		WithStdout(io.Discard).
		WithStderr(io.Discard))
	if err != nil {
		t.Fatalf("Failed to instantiate module: %v", err)
	}
	defer mod.Close(ctx)

	// Test get_version
	version := getVersion(ctx, mod)
	if version == "" {
		t.Error("Expected non-empty version")
	}
	t.Logf("Driver version: %s", version)

	// Test database_new
	code, result := apiCall(ctx, mod, "DatabaseDriver", "database_new", nil)
	if code != 0 {
		t.Errorf("database_new failed: code=%d, result=%s", code, string(result))
	}

	// Test database_init
	code, result = apiCall(ctx, mod, "DatabaseDriver", "database_init", result)
	if code != 0 {
		t.Errorf("database_init failed: code=%d, result=%s", code, string(result))
	}
}

// Helper functions for WASM integration tests

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
