// Package driver is the database/sql driver implementation for gosnowflake.
package driver

import sqldriver "database/sql/driver"

type Driver struct{}

func (d *Driver) Open(name string) (sqldriver.Conn, error) {
	return &snowflakeConn{}, nil
}

var _ sqldriver.Driver = (*Driver)(nil)
