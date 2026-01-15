//go:build !cgo

package snowflake

import (
	"context"
	"fmt"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// NativeBackend implements Backend using the native CGO library
// This is a stub when CGO is disabled.
type NativeBackend struct{}

// NewNativeBackend creates a new native backend (CGO disabled stub)
func NewNativeBackend(ctx context.Context) (*NativeBackend, error) {
	return nil, fmt.Errorf("native backend requires CGO; use WASM backend (backend=wasm) or rebuild with CGO_ENABLED=1")
}

func (b *NativeBackend) Initialize(ctx context.Context) error {
	return ErrNoBackend
}

func (b *NativeBackend) Close(ctx context.Context) error {
	return nil
}

func (b *NativeBackend) GetMemory() Memory {
	return nil
}

func (b *NativeBackend) DatabaseNew(ctx context.Context) (*pb.DatabaseHandle, error) {
	return nil, ErrNoBackend
}

func (b *NativeBackend) DatabaseInit(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return ErrNoBackend
}

func (b *NativeBackend) DatabaseRelease(ctx context.Context, dbHandle *pb.DatabaseHandle) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionNew(ctx context.Context) (*pb.ConnectionHandle, error) {
	return nil, ErrNoBackend
}

func (b *NativeBackend) ConnectionSetOptionString(ctx context.Context, connHandle *pb.ConnectionHandle, key, value string) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionSetOptionInt(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value int64) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionSetOptionDouble(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value float64) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionSetOptionBytes(ctx context.Context, connHandle *pb.ConnectionHandle, key string, value []byte) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionInit(ctx context.Context, connHandle *pb.ConnectionHandle, dbHandle *pb.DatabaseHandle) error {
	return ErrNoBackend
}

func (b *NativeBackend) ConnectionRelease(ctx context.Context, connHandle *pb.ConnectionHandle) error {
	return ErrNoBackend
}

func (b *NativeBackend) StatementNew(ctx context.Context, connHandle *pb.ConnectionHandle) (*pb.StatementHandle, error) {
	return nil, ErrNoBackend
}

func (b *NativeBackend) StatementSetSqlQuery(ctx context.Context, stmtHandle *pb.StatementHandle, query string) error {
	return ErrNoBackend
}

func (b *NativeBackend) StatementSetOptionString(ctx context.Context, stmtHandle *pb.StatementHandle, key, value string) error {
	return ErrNoBackend
}

func (b *NativeBackend) StatementBindStream(ctx context.Context, stmtHandle *pb.StatementHandle, stream []byte) error {
	return ErrNoBackend
}

func (b *NativeBackend) StatementExecuteQuery(ctx context.Context, stmtHandle *pb.StatementHandle) (*pb.ExecuteResult, error) {
	return nil, ErrNoBackend
}

func (b *NativeBackend) StatementRelease(ctx context.Context, stmtHandle *pb.StatementHandle) error {
	return ErrNoBackend
}

func (b *NativeBackend) ReleaseArrowResult(ctx context.Context, handle uint64) error {
	return nil
}
