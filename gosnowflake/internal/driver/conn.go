package driver

import (
	"database/sql/driver"
	"errors"
)

var errNotImplemented = errors.New("gosnowflake: not implemented")

type snowflakeConn struct{}

func (c *snowflakeConn) Prepare(query string) (driver.Stmt, error) {
	return nil, errNotImplemented
}

func (c *snowflakeConn) Close() error {
	return nil
}

func (c *snowflakeConn) Begin() (driver.Tx, error) {
	return nil, errNotImplemented
}

var _ driver.Conn = (*snowflakeConn)(nil)
