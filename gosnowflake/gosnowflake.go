// Package gosnowflake registers the database/sql driver name "gosnowflake".
package gosnowflake

import (
	"database/sql"

	"go.snowflake.com/drivers/gosnowflake/internal/driver"
)

func init() {
	sql.Register("gosnowflake", &driver.Driver{})
}
