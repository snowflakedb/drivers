// Package snowflake provides a database/sql driver for Snowflake.
//
// This is the universal driver implementation that supports both
// native (CGO) and WASM backends.
//
// Basic usage:
//
//	import (
//	    "database/sql"
//	    _ "github.com/snowflakedb/universal-driver/go/snowflake"
//	)
//
//	func main() {
//	    db, err := sql.Open("snowflake", "user:password@account/database/schema")
//	    if err != nil {
//	        log.Fatal(err)
//	    }
//	    defer db.Close()
//	}
//
// DSN Format:
//
// The DSN (Data Source Name) format is:
//
//	user[:password]@account[/database[/schema]][?param1=value1&param2=value2]
//
// Or using key=value format:
//
//	account=<account>&user=<user>&password=<password>&database=<database>
//
// Connection Parameters:
//   - account: Snowflake account identifier (required)
//   - user: Login user name (required)
//   - password: Login password
//   - database: Default database
//   - schema: Default schema
//   - warehouse: Default warehouse
//   - role: Default role
//   - authenticator: Authentication method (SNOWFLAKE, SNOWFLAKE_JWT, EXTERNALBROWSER, etc.)
//   - private_key: PEM-encoded private key for JWT authentication
//   - private_key_file: Path to private key file
//   - private_key_password: Password for encrypted private key
//   - host: Snowflake host (optional, derived from account)
//   - port: Snowflake port (default: 443)
//   - protocol: Connection protocol (default: https)
package snowflake
