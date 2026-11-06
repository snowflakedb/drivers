package gosnowflake_test

import (
	"database/sql"
	"testing"

	_ "go.snowflake.com/drivers/gosnowflake"
)

func TestSQLOpenPrepareClose(t *testing.T) {
	db, err := sql.Open("gosnowflake", "user:password@account.snowflakecomputing.com/db")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := db.Close(); err != nil {
			t.Error(err)
		}
	})

	_, err = db.Prepare("select 1")
	if err == nil || err.Error() != "gosnowflake: not implemented" {
		t.Fatalf("Prepare() error = %v, want gosnowflake: not implemented", err)
	}
}
