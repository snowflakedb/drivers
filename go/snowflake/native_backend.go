//go:build cgo

package snowflake

/*
#cgo LDFLAGS: -ldl
#include <dlfcn.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

// Function pointer types
typedef uint32_t (*sf_core_api_call_proto_t)(
    const char* api,
    const char* method,
    const uint8_t* request,
    size_t request_len,
    uint8_t** response,
    size_t* response_len
);

typedef void (*sf_core_free_proto_response_t)(uint8_t* response, size_t response_len);

// Global function pointers
static void* lib_handle = NULL;
static sf_core_api_call_proto_t api_call_fn = NULL;
static sf_core_free_proto_response_t free_response_fn = NULL;

// Load the library
int load_sf_core(const char* path) {
    lib_handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!lib_handle) {
        return -1;
    }

    api_call_fn = (sf_core_api_call_proto_t)dlsym(lib_handle, "sf_core_api_call_proto");
    if (!api_call_fn) {
        dlclose(lib_handle);
        lib_handle = NULL;
        return -2;
    }

    free_response_fn = (sf_core_free_proto_response_t)dlsym(lib_handle, "sf_core_free_proto_response");
    // free_response_fn is optional

    return 0;
}

// Unload the library
void unload_sf_core() {
    if (lib_handle) {
        dlclose(lib_handle);
        lib_handle = NULL;
        api_call_fn = NULL;
        free_response_fn = NULL;
    }
}

// Call the API
uint32_t call_api(const char* api, const char* method,
                  const uint8_t* request, size_t request_len,
                  uint8_t** response, size_t* response_len) {
    if (!api_call_fn) {
        return 999; // Not initialized
    }
    return api_call_fn(api, method, request, request_len, response, response_len);
}

// Free response
void free_response(uint8_t* response, size_t response_len) {
    if (free_response_fn && response) {
        free_response_fn(response, response_len);
    }
}

const char* get_dl_error() {
    return dlerror();
}
*/
import "C"

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"unsafe"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
	"google.golang.org/protobuf/proto"
)

const (
	nativeResultOK             = 0
	nativeResultError          = 1
	nativeResultTransportError = 2
)

// NativeBackend implements Backend using the native CGO library
type NativeBackend struct {
	mu     sync.Mutex
	loaded bool
}

// NewNativeBackend creates a new native backend
func NewNativeBackend(ctx context.Context) (*NativeBackend, error) {
	libPath := findNativeLibrary()
	if libPath == "" {
		return nil, fmt.Errorf("native library (libsf_core) not found")
	}

	cPath := C.CString(libPath)
	defer C.free(unsafe.Pointer(cPath))

	result := C.load_sf_core(cPath)
	if result != 0 {
		errMsg := C.GoString(C.get_dl_error())
		return nil, fmt.Errorf("failed to load native library %s: %s (code: %d)", libPath, errMsg, result)
	}

	return &NativeBackend{loaded: true}, nil
}

// findNativeLibrary searches for libsf_core
func findNativeLibrary() string {
	var libName string
	switch {
	case fileExists("/etc/os-release"): // Linux
		libName = "libsf_core.so"
	default: // macOS
		libName = "libsf_core.dylib"
	}

	candidates := []string{
		libName,
		filepath.Join("target", "release", libName),
		filepath.Join("..", "target", "release", libName),
		filepath.Join("..", "..", "target", "release", libName),
	}

	if envPath := os.Getenv("SNOWFLAKE_NATIVE_LIB_PATH"); envPath != "" {
		candidates = append([]string{envPath}, candidates...)
	}

	for _, path := range candidates {
		if fileExists(path) {
			return path
		}
	}
	return ""
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func (b *NativeBackend) Initialize(ctx context.Context) error {
	return nil
}

func (b *NativeBackend) Close(ctx context.Context) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.loaded {
		C.unload_sf_core()
		b.loaded = false
	}
	return nil
}

func (b *NativeBackend) GetMemory() Memory {
	return nil // Native backend doesn't need memory access
}

// callProto calls a protobuf API method on the native library
func (b *NativeBackend) callProto(ctx context.Context, apiName, method string, req proto.Message, resp proto.Message) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	if !b.loaded {
		return ErrNoBackend
	}

	// Serialize request
	var reqBytes []byte
	var err error
	if req != nil {
		reqBytes, err = proto.Marshal(req)
		if err != nil {
			return fmt.Errorf("failed to marshal request: %w", err)
		}
	}

	cApi := C.CString(apiName)
	defer C.free(unsafe.Pointer(cApi))

	cMethod := C.CString(method)
	defer C.free(unsafe.Pointer(cMethod))

	var cReq *C.uint8_t
	var cReqLen C.size_t
	if len(reqBytes) > 0 {
		cReq = (*C.uint8_t)(unsafe.Pointer(&reqBytes[0]))
		cReqLen = C.size_t(len(reqBytes))
	}

	var cResp *C.uint8_t
	var cRespLen C.size_t

	result := C.call_api(cApi, cMethod, cReq, cReqLen, &cResp, &cRespLen)

	// Copy response before freeing
	var respBytes []byte
	if cRespLen > 0 && cResp != nil {
		respBytes = C.GoBytes(unsafe.Pointer(cResp), C.int(cRespLen))
		C.free_response(cResp, cRespLen)
	}

	switch result {
	case nativeResultOK:
		if resp != nil && len(respBytes) > 0 {
			if err := proto.Unmarshal(respBytes, resp); err != nil {
				return fmt.Errorf("failed to unmarshal response: %w", err)
			}
		}
		return nil
	case nativeResultError:
		var exc pb.DriverException
		if err := proto.Unmarshal(respBytes, &exc); err != nil {
			return fmt.Errorf("driver error (failed to parse): %v", respBytes)
		}
		return &SnowflakeError{
			Code:    int(exc.GetStatusCode()),
			Message: exc.GetMessage(),
		}
	case nativeResultTransportError:
		return fmt.Errorf("transport error: %s", string(respBytes))
	default:
		return fmt.Errorf("unknown result code: %d", result)
	}
}

// DatabaseNew implements Backend
func (b *NativeBackend) DatabaseNew(ctx context.Context) (*pb.DatabaseHandle, error) {
	resp := &pb.DatabaseNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "database_new", &pb.DatabaseNewRequest{}, resp); err != nil {
		return nil, err
	}
	return resp.GetDbHandle(), nil
}

// DatabaseInit implements Backend
func (b *NativeBackend) DatabaseInit(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "database_init", &pb.DatabaseInitRequest{DbHandle: dbHandle}, &pb.DatabaseInitResponse{})
}

// DatabaseRelease implements Backend
func (b *NativeBackend) DatabaseRelease(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "database_release", &pb.DatabaseReleaseRequest{DbHandle: dbHandle}, &pb.DatabaseReleaseResponse{})
}

// ConnectionNew implements Backend
func (b *NativeBackend) ConnectionNew(ctx context.Context) (*pb.ConnectionHandle, error) {
	resp := &pb.ConnectionNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "connection_new", &pb.ConnectionNewRequest{}, resp); err != nil {
		return nil, err
	}
	return resp.GetConnHandle(), nil
}

// ConnectionSetOptionString implements Backend
func (b *NativeBackend) ConnectionSetOptionString(ctx context.Context, connHandle *pb.ConnectionHandle, key, value string) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_string",
		&pb.ConnectionSetOptionStringRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionStringResponse{})
}

// ConnectionSetOptionInt implements Backend
func (b *NativeBackend) ConnectionSetOptionInt(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value int64) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_int",
		&pb.ConnectionSetOptionIntRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionIntResponse{})
}

// ConnectionSetOptionDouble implements Backend
func (b *NativeBackend) ConnectionSetOptionDouble(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value float64) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_double",
		&pb.ConnectionSetOptionDoubleRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionDoubleResponse{})
}

// ConnectionSetOptionBytes implements Backend
func (b *NativeBackend) ConnectionSetOptionBytes(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value []byte) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_set_option_bytes",
		&pb.ConnectionSetOptionBytesRequest{ConnHandle: connHandle, Key: key, Value: value},
		&pb.ConnectionSetOptionBytesResponse{})
}

// ConnectionInit implements Backend
func (b *NativeBackend) ConnectionInit(ctx context.Context, connHandle *pb.ConnectionHandle, dbHandle *pb.DatabaseHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_init",
		&pb.ConnectionInitRequest{ConnHandle: connHandle, DbHandle: dbHandle},
		&pb.ConnectionInitResponse{})
}

// ConnectionRelease implements Backend
func (b *NativeBackend) ConnectionRelease(ctx context.Context, connHandle *pb.ConnectionHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "connection_release",
		&pb.ConnectionReleaseRequest{ConnHandle: connHandle},
		&pb.ConnectionReleaseResponse{})
}

// StatementNew implements Backend
func (b *NativeBackend) StatementNew(ctx context.Context, connHandle *pb.ConnectionHandle) (*pb.StatementHandle, error) {
	resp := &pb.StatementNewResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "statement_new",
		&pb.StatementNewRequest{ConnHandle: connHandle}, resp); err != nil {
		return nil, err
	}
	return resp.GetStmtHandle(), nil
}

// StatementSetSqlQuery implements Backend
func (b *NativeBackend) StatementSetSqlQuery(ctx context.Context, stmtHandle *pb.StatementHandle, query string) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_set_sql_query",
		&pb.StatementSetSqlQueryRequest{StmtHandle: stmtHandle, Query: query},
		&pb.StatementSetSqlQueryResponse{})
}

// StatementSetOptionString implements Backend
func (b *NativeBackend) StatementSetOptionString(ctx context.Context, stmtHandle *pb.StatementHandle, key, value string) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_set_option_string",
		&pb.StatementSetOptionStringRequest{StmtHandle: stmtHandle, Key: key, Value: value},
		&pb.StatementSetOptionStringResponse{})
}

// StatementBindStream implements Backend
func (b *NativeBackend) StatementBindStream(ctx context.Context, stmtHandle *pb.StatementHandle, stream []byte) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_bind_stream",
		&pb.StatementBindStreamRequest{StmtHandle: stmtHandle, Stream: stream},
		&pb.StatementBindStreamResponse{})
}

// StatementExecuteQuery implements Backend
func (b *NativeBackend) StatementExecuteQuery(ctx context.Context, stmtHandle *pb.StatementHandle) (*pb.ExecuteResult, error) {
	resp := &pb.StatementExecuteQueryResponse{}
	if err := b.callProto(ctx, "DatabaseDriver", "statement_execute_query",
		&pb.StatementExecuteQueryRequest{StmtHandle: stmtHandle}, resp); err != nil {
		return nil, err
	}
	return resp.GetResult(), nil
}

// StatementRelease implements Backend
func (b *NativeBackend) StatementRelease(ctx context.Context, stmtHandle *pb.StatementHandle) error {
	return b.callProto(ctx, "DatabaseDriver", "statement_release",
		&pb.StatementReleaseRequest{StmtHandle: stmtHandle},
		&pb.StatementReleaseResponse{})
}

// ReleaseArrowResult implements Backend
func (b *NativeBackend) ReleaseArrowResult(ctx context.Context, handle uint64) error {
	return nil // Not needed for native backend - memory is in native space
}
