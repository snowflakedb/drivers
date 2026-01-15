//go:build cgo

package snowflake

/*
#include <stdint.h>

// Arrow C Data Interface structures
struct ArrowSchema {
	const char* format;
	const char* name;
	const char* metadata;
	int64_t flags;
	int64_t n_children;
	struct ArrowSchema** children;
	struct ArrowSchema* dictionary;
	void (*release)(struct ArrowSchema*);
	void* private_data;
};

struct ArrowArray {
	int64_t length;
	int64_t null_count;
	int64_t offset;
	int64_t n_buffers;
	int64_t n_children;
	const void** buffers;
	struct ArrowArray** children;
	struct ArrowArray* dictionary;
	void (*release)(struct ArrowArray*);
	void* private_data;
};

struct ArrowArrayStream {
	int (*get_schema)(struct ArrowArrayStream*, struct ArrowSchema* out);
	int (*get_next)(struct ArrowArrayStream*, struct ArrowArray* out);
	const char* (*get_last_error)(struct ArrowArrayStream*);
	void (*release)(struct ArrowArrayStream*);
	void* private_data;
};

// Helper to get ArrowArrayStream from pointer
struct ArrowArrayStream* get_stream_from_bytes(uint8_t* bytes) {
	return *((struct ArrowArrayStream**)bytes);
}
*/
import "C"

import (
	"context"
	"fmt"
	"unsafe"

	"github.com/apache/arrow-go/v18/arrow"
	"github.com/apache/arrow-go/v18/arrow/arrio"
	"github.com/apache/arrow-go/v18/arrow/cdata"
	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// ffiReaderWrapper wraps arrio.Reader to implement ffiReaderInterface
type ffiReaderWrapper struct {
	reader arrio.Reader
	schema *arrow.Schema
}

func (w *ffiReaderWrapper) Read() (arrow.RecordBatch, error) {
	return w.reader.Read()
}

func (w *ffiReaderWrapper) Release() {
	// arrio.Reader doesn't have a Release method, but the underlying stream
	// will be released when we're done reading
}

// newRowsFromFFI creates Rows from a native FFI ArrowArrayStream pointer
func newRowsFromFFI(ctx context.Context, backend Backend, result *pb.ExecuteResult) (*Rows, error) {
	streamPtr := result.GetStream()
	if streamPtr == nil || len(streamPtr.GetValue()) == 0 {
		return nil, fmt.Errorf("no FFI stream pointer in result")
	}

	// Convert protobuf bytes to C stream pointer
	valueBytes := streamPtr.GetValue()
	cStream := C.get_stream_from_bytes((*C.uint8_t)(unsafe.Pointer(&valueBytes[0])))
	if cStream == nil {
		return nil, fmt.Errorf("invalid FFI stream pointer")
	}

	// Import the stream using Arrow's cdata package
	reader, err := cdata.ImportCRecordReader((*cdata.CArrowArrayStream)(unsafe.Pointer(cStream)), nil)
	if err != nil {
		return nil, fmt.Errorf("failed to import FFI stream: %w", err)
	}

	// Get schema from the first record batch
	rec, err := reader.Read()
	if err != nil {
		return nil, fmt.Errorf("failed to read first record: %w", err)
	}

	schema := rec.Schema()

	rows := &Rows{
		backend:      backend,
		result:       result,
		schema:       schema,
		ctx:          ctx,
		ffiReader:    &ffiReaderWrapper{reader: reader, schema: schema},
		currentBatch: rec,
		currentRow:   0,
	}

	return rows, nil
}
