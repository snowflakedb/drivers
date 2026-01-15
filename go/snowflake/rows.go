package snowflake

import (
	"bytes"
	"context"
	"database/sql/driver"
	"fmt"
	"io"
	"reflect"

	"github.com/apache/arrow-go/v18/arrow"
	"github.com/apache/arrow-go/v18/arrow/array"
	"github.com/apache/arrow-go/v18/arrow/ipc"
	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// Rows implements database/sql/driver.Rows
type Rows struct {
	backend       Backend
	result        *pb.ExecuteResult
	schema        *arrow.Schema
	reader        *ipc.Reader
	ffiReader     ffiReaderInterface // For native FFI results (CGO only)
	currentBatch  arrow.Record
	currentRow    int
	releaseHandle uint64
	closed        bool
	ctx           context.Context
}

// ffiReaderInterface is the interface for FFI arrow readers
type ffiReaderInterface interface {
	Read() (arrow.RecordBatch, error)
	Release()
}

var (
	_ driver.Rows                           = (*Rows)(nil)
	_ driver.RowsColumnTypeDatabaseTypeName = (*Rows)(nil)
	_ driver.RowsColumnTypeNullable         = (*Rows)(nil)
	_ driver.RowsColumnTypeScanType         = (*Rows)(nil)
)

// NewRows creates a new Rows from an ExecuteResult
func NewRows(ctx context.Context, backend Backend, result *pb.ExecuteResult) (*Rows, error) {
	// Handle WASM arrow result (zero-copy via memory offsets)
	if wasmResult := result.GetWasmResult(); wasmResult != nil {
		rows := &Rows{
			backend:       backend,
			result:        result,
			ctx:           ctx,
			releaseHandle: wasmResult.GetReleaseHandle(),
		}

		// Parse schema from IPC
		if len(wasmResult.GetSchemaIpc()) > 0 {
			reader, err := ipc.NewReader(bytes.NewReader(wasmResult.GetSchemaIpc()))
			if err != nil {
				return nil, fmt.Errorf("failed to parse schema IPC: %w", err)
			}
			rows.schema = reader.Schema()
			rows.reader = reader
		}
		return rows, nil
	}

	// Handle native arrow result (FFI pointer)
	if streamPtr := result.GetStream(); streamPtr != nil && len(streamPtr.GetValue()) > 0 {
		return newRowsFromFFI(ctx, backend, result)
	}

	return nil, fmt.Errorf("no arrow result in execute response")
}

// Columns implements driver.Rows
func (r *Rows) Columns() []string {
	if r.schema == nil {
		return nil
	}
	cols := make([]string, r.schema.NumFields())
	for i := 0; i < r.schema.NumFields(); i++ {
		cols[i] = r.schema.Field(i).Name
	}
	return cols
}

// Close implements driver.Rows
func (r *Rows) Close() error {
	if r.closed {
		return nil
	}
	r.closed = true

	if r.currentBatch != nil {
		r.currentBatch.Release()
		r.currentBatch = nil
	}

	if r.reader != nil {
		r.reader.Release()
		r.reader = nil
	}

	if r.ffiReader != nil {
		r.ffiReader.Release()
		r.ffiReader = nil
	}

	// Release WASM memory if needed
	if r.releaseHandle != 0 && r.backend != nil {
		r.backend.ReleaseArrowResult(r.ctx, r.releaseHandle)
	}

	return nil
}

// Next implements driver.Rows
func (r *Rows) Next(dest []driver.Value) error {
	if r.closed {
		return io.EOF
	}

	// Load next batch if needed
	for r.currentBatch == nil || r.currentRow >= int(r.currentBatch.NumRows()) {
		if r.currentBatch != nil {
			r.currentBatch.Release()
			r.currentBatch = nil
		}

		// Read next batch from IPC reader or FFI reader
		if r.reader != nil && r.reader.Next() {
			r.currentBatch = r.reader.Record()
			r.currentBatch.Retain()
			r.currentRow = 0
		} else if r.ffiReader != nil {
			rec, err := r.ffiReader.Read()
			if err != nil || rec == nil {
				return io.EOF
			}
			r.currentBatch = rec
			r.currentBatch.Retain()
			r.currentRow = 0
		} else {
			return io.EOF
		}
	}

	// Extract values from current row
	for i := 0; i < int(r.currentBatch.NumCols()); i++ {
		col := r.currentBatch.Column(i)
		dest[i] = extractValue(col, r.currentRow)
	}

	r.currentRow++
	return nil
}

// extractValue extracts a Go value from an Arrow array at the given index
func extractValue(col arrow.Array, row int) interface{} {
	if col.IsNull(row) {
		return nil
	}

	switch arr := col.(type) {
	case *array.Int8:
		return arr.Value(row)
	case *array.Int16:
		return arr.Value(row)
	case *array.Int32:
		return arr.Value(row)
	case *array.Int64:
		return arr.Value(row)
	case *array.Uint8:
		return arr.Value(row)
	case *array.Uint16:
		return arr.Value(row)
	case *array.Uint32:
		return arr.Value(row)
	case *array.Uint64:
		return arr.Value(row)
	case *array.Float32:
		return arr.Value(row)
	case *array.Float64:
		return arr.Value(row)
	case *array.String:
		return arr.Value(row)
	case *array.LargeString:
		return arr.Value(row)
	case *array.Binary:
		return arr.Value(row)
	case *array.LargeBinary:
		return arr.Value(row)
	case *array.Boolean:
		return arr.Value(row)
	case *array.Date32:
		return arr.Value(row).ToTime()
	case *array.Date64:
		return arr.Value(row).ToTime()
	case *array.Timestamp:
		return arr.Value(row).ToTime(arr.DataType().(*arrow.TimestampType).Unit)
	case *array.Decimal128:
		return arr.Value(row).ToString(arr.DataType().(*arrow.Decimal128Type).Scale)
	case *array.Decimal256:
		return arr.Value(row).ToString(arr.DataType().(*arrow.Decimal256Type).Scale)
	default:
		// Fallback: convert to string
		return fmt.Sprintf("%v", col.ValueStr(row))
	}
}

// ColumnTypeDatabaseTypeName implements driver.RowsColumnTypeDatabaseTypeName
func (r *Rows) ColumnTypeDatabaseTypeName(index int) string {
	if r.schema == nil || index >= r.schema.NumFields() {
		return ""
	}
	field := r.schema.Field(index)
	return arrowTypeToSnowflakeType(field.Type)
}

// ColumnTypeNullable implements driver.RowsColumnTypeNullable
func (r *Rows) ColumnTypeNullable(index int) (nullable, ok bool) {
	if r.schema == nil || index >= r.schema.NumFields() {
		return false, false
	}
	field := r.schema.Field(index)
	return field.Nullable, true
}

// ColumnTypeScanType implements driver.RowsColumnTypeScanType
func (r *Rows) ColumnTypeScanType(index int) reflect.Type {
	if r.schema == nil || index >= r.schema.NumFields() {
		return reflect.TypeOf((*interface{})(nil)).Elem()
	}
	field := r.schema.Field(index)
	return arrowTypeToGoType(field.Type)
}

// arrowTypeToSnowflakeType converts Arrow type to Snowflake type name
func arrowTypeToSnowflakeType(dt arrow.DataType) string {
	switch dt.ID() {
	case arrow.INT8, arrow.INT16, arrow.INT32, arrow.INT64:
		return "INTEGER"
	case arrow.UINT8, arrow.UINT16, arrow.UINT32, arrow.UINT64:
		return "INTEGER"
	case arrow.FLOAT32:
		return "FLOAT"
	case arrow.FLOAT64:
		return "DOUBLE"
	case arrow.STRING, arrow.LARGE_STRING:
		return "VARCHAR"
	case arrow.BINARY, arrow.LARGE_BINARY:
		return "BINARY"
	case arrow.BOOL:
		return "BOOLEAN"
	case arrow.DATE32, arrow.DATE64:
		return "DATE"
	case arrow.TIMESTAMP:
		return "TIMESTAMP"
	case arrow.TIME32, arrow.TIME64:
		return "TIME"
	case arrow.DECIMAL128, arrow.DECIMAL256:
		return "NUMBER"
	default:
		return "VARIANT"
	}
}

// arrowTypeToGoType converts Arrow type to Go reflect.Type
func arrowTypeToGoType(dt arrow.DataType) reflect.Type {
	switch dt.ID() {
	case arrow.INT8:
		return reflect.TypeOf(int8(0))
	case arrow.INT16:
		return reflect.TypeOf(int16(0))
	case arrow.INT32:
		return reflect.TypeOf(int32(0))
	case arrow.INT64:
		return reflect.TypeOf(int64(0))
	case arrow.UINT8:
		return reflect.TypeOf(uint8(0))
	case arrow.UINT16:
		return reflect.TypeOf(uint16(0))
	case arrow.UINT32:
		return reflect.TypeOf(uint32(0))
	case arrow.UINT64:
		return reflect.TypeOf(uint64(0))
	case arrow.FLOAT32:
		return reflect.TypeOf(float32(0))
	case arrow.FLOAT64:
		return reflect.TypeOf(float64(0))
	case arrow.STRING, arrow.LARGE_STRING:
		return reflect.TypeOf("")
	case arrow.BINARY, arrow.LARGE_BINARY:
		return reflect.TypeOf([]byte{})
	case arrow.BOOL:
		return reflect.TypeOf(false)
	default:
		return reflect.TypeOf((*interface{})(nil)).Elem()
	}
}
