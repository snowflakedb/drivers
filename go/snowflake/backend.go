package snowflake

import (
	"context"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// Backend is the interface for driver backend implementations (WASM or native)
type Backend interface {
	// Initialize initializes the backend
	Initialize(ctx context.Context) error

	// Close cleans up backend resources
	Close(ctx context.Context) error

	// DatabaseNew creates a new database handle
	DatabaseNew(ctx context.Context) (*pb.DatabaseHandle, error)

	// DatabaseInit initializes the database
	DatabaseInit(ctx context.Context, dbHandle *pb.DatabaseHandle) error

	// DatabaseRelease releases the database handle
	DatabaseRelease(ctx context.Context, dbHandle *pb.DatabaseHandle) error

	// ConnectionNew creates a new connection handle
	ConnectionNew(ctx context.Context) (*pb.ConnectionHandle, error)

	// ConnectionSetOptionString sets a string option on the connection
	ConnectionSetOptionString(ctx context.Context, connHandle *pb.ConnectionHandle, key, value string) error

	// ConnectionSetOptionInt sets an int option on the connection
	ConnectionSetOptionInt(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value int64) error

	// ConnectionSetOptionDouble sets a double option on the connection
	ConnectionSetOptionDouble(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value float64) error

	// ConnectionSetOptionBytes sets a bytes option on the connection
	ConnectionSetOptionBytes(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value []byte) error

	// ConnectionInit initializes the connection (performs login)
	ConnectionInit(ctx context.Context, connHandle *pb.ConnectionHandle, dbHandle *pb.DatabaseHandle) error

	// ConnectionRelease releases the connection handle
	ConnectionRelease(ctx context.Context, connHandle *pb.ConnectionHandle) error

	// StatementNew creates a new statement handle
	StatementNew(ctx context.Context, connHandle *pb.ConnectionHandle) (*pb.StatementHandle, error)

	// StatementSetSqlQuery sets the SQL query for the statement
	StatementSetSqlQuery(ctx context.Context, stmtHandle *pb.StatementHandle, query string) error

	// StatementSetOptionString sets a string option on the statement
	StatementSetOptionString(ctx context.Context, stmtHandle *pb.StatementHandle, key, value string) error

	// StatementBindStream binds parameters to the statement using a byte stream
	StatementBindStream(ctx context.Context, stmtHandle *pb.StatementHandle, stream []byte) error

	// StatementExecuteQuery executes the query and returns the result
	StatementExecuteQuery(ctx context.Context, stmtHandle *pb.StatementHandle) (*pb.ExecuteResult, error)

	// StatementRelease releases the statement handle
	StatementRelease(ctx context.Context, stmtHandle *pb.StatementHandle) error

	// ReleaseArrowResult releases arrow result memory (WASM only)
	ReleaseArrowResult(ctx context.Context, handle uint64) error

	// GetMemory returns the WASM memory for zero-copy access (WASM only, returns nil for native)
	GetMemory() Memory
}

// Memory provides access to backend memory for zero-copy arrow access
type Memory interface {
	Read(offset, length uint32) ([]byte, bool)
}

var (
	// globalBackend is the shared backend instance
	globalBackend Backend
)

// GetBackend returns the global backend instance, initializing if needed
func GetBackend(ctx context.Context, backendType string) (Backend, error) {
	if globalBackend != nil {
		return globalBackend, nil
	}

	var backend Backend
	var err error

	switch backendType {
	case "wasm":
		backend, err = NewWASMBackend(ctx)
	case "native":
		backend, err = NewNativeBackend(ctx)
	default:
		// Try WASM first, then native
		backend, err = NewWASMBackend(ctx)
		if err != nil {
			backend, err = NewNativeBackend(ctx)
		}
	}

	if err != nil {
		return nil, err
	}

	globalBackend = backend
	return globalBackend, nil
}

// ResetBackend resets the global backend (for testing)
func ResetBackend() {
	if globalBackend != nil {
		globalBackend.Close(context.Background())
		globalBackend = nil
	}
}
