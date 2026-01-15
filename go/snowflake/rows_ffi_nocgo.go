//go:build !cgo

package snowflake

import (
	"context"
	"fmt"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// newRowsFromFFI is not available without CGO
func newRowsFromFFI(ctx context.Context, backend Backend, result *pb.ExecuteResult) (*Rows, error) {
	return nil, fmt.Errorf("native FFI arrow results require CGO; use WASM backend instead")
}
