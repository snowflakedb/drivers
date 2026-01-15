package snowflake

import (
	"context"
	"database/sql/driver"
	"encoding/json"
	"fmt"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// Stmt implements database/sql/driver.Stmt
type Stmt struct {
	conn       *Conn
	stmtHandle *pb.StatementHandle
	query      string
	closed     bool
}

var (
	_ driver.Stmt             = (*Stmt)(nil)
	_ driver.StmtExecContext  = (*Stmt)(nil)
	_ driver.StmtQueryContext = (*Stmt)(nil)
)

// Close implements driver.Stmt
func (s *Stmt) Close() error {
	if s.closed {
		return nil
	}
	s.closed = true

	if s.stmtHandle != nil {
		return s.conn.backend.StatementRelease(context.Background(), s.stmtHandle)
	}
	return nil
}

// NumInput implements driver.Stmt
func (s *Stmt) NumInput() int {
	// Return -1 to indicate we don't know the number of parameters
	return -1
}

// Exec implements driver.Stmt (deprecated, use ExecContext)
func (s *Stmt) Exec(args []driver.Value) (driver.Result, error) {
	namedArgs := make([]driver.NamedValue, len(args))
	for i, arg := range args {
		namedArgs[i] = driver.NamedValue{
			Ordinal: i + 1,
			Value:   arg,
		}
	}
	return s.ExecContext(context.Background(), namedArgs)
}

// ExecContext implements driver.StmtExecContext
func (s *Stmt) ExecContext(ctx context.Context, args []driver.NamedValue) (driver.Result, error) {
	if s.closed {
		return nil, driver.ErrBadConn
	}

	// Bind parameters if any
	if len(args) > 0 {
		if err := s.bindParameters(ctx, args); err != nil {
			return nil, err
		}
	}

	result, err := s.conn.backend.StatementExecuteQuery(ctx, s.stmtHandle)
	if err != nil {
		return nil, err
	}

	return &Result{
		rowsAffected: result.GetRowsAffected(),
	}, nil
}

// Query implements driver.Stmt (deprecated, use QueryContext)
func (s *Stmt) Query(args []driver.Value) (driver.Rows, error) {
	namedArgs := make([]driver.NamedValue, len(args))
	for i, arg := range args {
		namedArgs[i] = driver.NamedValue{
			Ordinal: i + 1,
			Value:   arg,
		}
	}
	return s.QueryContext(context.Background(), namedArgs)
}

// QueryContext implements driver.StmtQueryContext
func (s *Stmt) QueryContext(ctx context.Context, args []driver.NamedValue) (driver.Rows, error) {
	if s.closed {
		return nil, driver.ErrBadConn
	}

	// Bind parameters if any
	if len(args) > 0 {
		if err := s.bindParameters(ctx, args); err != nil {
			return nil, err
		}
	}

	result, err := s.conn.backend.StatementExecuteQuery(ctx, s.stmtHandle)
	if err != nil {
		return nil, err
	}

	return NewRows(ctx, s.conn.backend, result)
}

// bindParameters converts Go parameters to JSON and binds them
func (s *Stmt) bindParameters(ctx context.Context, args []driver.NamedValue) error {
	bindings := make(map[string]bindParam)

	for _, arg := range args {
		// Use ordinal position as key (1-based)
		key := fmt.Sprintf("%d", arg.Ordinal)
		if arg.Name != "" {
			key = arg.Name
		}

		param, err := goValueToBindParam(arg.Value)
		if err != nil {
			return fmt.Errorf("failed to convert parameter %s: %w", key, err)
		}
		bindings[key] = param
	}

	// Serialize to JSON
	jsonBytes, err := json.Marshal(bindings)
	if err != nil {
		return fmt.Errorf("failed to serialize parameters: %w", err)
	}

	return s.conn.backend.StatementBindStream(ctx, s.stmtHandle, jsonBytes)
}

// bindParam matches the Rust BindParameter struct
type bindParam struct {
	Type  string      `json:"type"`
	Value interface{} `json:"value"`
}

// goValueToBindParam converts a Go value to a Snowflake bind parameter
func goValueToBindParam(v interface{}) (bindParam, error) {
	switch val := v.(type) {
	case nil:
		return bindParam{Type: "TEXT", Value: nil}, nil
	case int, int8, int16, int32, int64:
		return bindParam{Type: "FIXED", Value: fmt.Sprintf("%d", val)}, nil
	case uint, uint8, uint16, uint32, uint64:
		return bindParam{Type: "FIXED", Value: fmt.Sprintf("%d", val)}, nil
	case float32, float64:
		return bindParam{Type: "REAL", Value: fmt.Sprintf("%v", val)}, nil
	case string:
		return bindParam{Type: "TEXT", Value: val}, nil
	case bool:
		return bindParam{Type: "BOOLEAN", Value: val}, nil
	case []byte:
		return bindParam{Type: "BINARY", Value: fmt.Sprintf("%X", val)}, nil
	default:
		// Try to convert to string
		return bindParam{Type: "TEXT", Value: fmt.Sprintf("%v", val)}, nil
	}
}

// Result implements database/sql/driver.Result
type Result struct {
	rowsAffected int64
	lastInsertId int64
}

// LastInsertId implements driver.Result
func (r *Result) LastInsertId() (int64, error) {
	return r.lastInsertId, nil
}

// RowsAffected implements driver.Result
func (r *Result) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}
