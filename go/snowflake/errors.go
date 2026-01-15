package snowflake

import (
	"errors"
	"fmt"
)

var (
	// ErrEmptyAccount indicates the account parameter is empty
	ErrEmptyAccount = errors.New("account is empty")
	// ErrEmptyUser indicates the user parameter is empty
	ErrEmptyUser = errors.New("user is empty")
	// ErrEmptyPassword indicates the password parameter is empty when required
	ErrEmptyPassword = errors.New("password is empty")
	// ErrInvalidDSN indicates the DSN format is invalid
	ErrInvalidDSN = errors.New("invalid DSN format")
	// ErrConnectionClosed indicates the connection is already closed
	ErrConnectionClosed = errors.New("connection is closed")
	// ErrQueryCanceled indicates the query was canceled
	ErrQueryCanceled = errors.New("query canceled")
	// ErrNoBackend indicates no backend is available
	ErrNoBackend = errors.New("no backend available")
	// ErrAuthenticationFailed indicates authentication failed
	ErrAuthenticationFailed = errors.New("authentication failed")
)

// SnowflakeError represents a Snowflake-specific error
type SnowflakeError struct {
	Code    int
	Message string
	SQLState string
	QueryID  string
}

func (e *SnowflakeError) Error() string {
	if e.QueryID != "" {
		return fmt.Sprintf("Snowflake error %d (%s): %s [QueryID: %s]", e.Code, e.SQLState, e.Message, e.QueryID)
	}
	return fmt.Sprintf("Snowflake error %d (%s): %s", e.Code, e.SQLState, e.Message)
}

// AuthError represents an authentication error
type AuthError struct {
	Message string
	Cause   error
}

func (e *AuthError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("authentication error: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("authentication error: %s", e.Message)
}

func (e *AuthError) Unwrap() error {
	return e.Cause
}

// ConnectionError represents a connection error
type ConnectionError struct {
	Message string
	Cause   error
}

func (e *ConnectionError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("connection error: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("connection error: %s", e.Message)
}

func (e *ConnectionError) Unwrap() error {
	return e.Cause
}

// QueryError represents a query execution error
type QueryError struct {
	Message string
	QueryID string
	Cause   error
}

func (e *QueryError) Error() string {
	if e.QueryID != "" {
		return fmt.Sprintf("query error [%s]: %s", e.QueryID, e.Message)
	}
	return fmt.Sprintf("query error: %s", e.Message)
}

func (e *QueryError) Unwrap() error {
	return e.Cause
}
