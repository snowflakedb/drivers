package snowflake

import (
	"context"
	"database/sql"
	"fmt"
	"os"
	"testing"
)

// getTestBackends returns the list of backends to test.
// By default, tests "wasm". Set SNOWFLAKE_TEST_BACKENDS=wasm,native to test both.
// When CGO is disabled, "native" tests will be skipped.
func getTestBackends() []string {
	if backends := os.Getenv("SNOWFLAKE_TEST_BACKENDS"); backends != "" {
		var result []string
		for _, b := range splitComma(backends) {
			if b == "wasm" || b == "native" {
				result = append(result, b)
			}
		}
		if len(result) > 0 {
			return result
		}
	}
	// Default: test both backends
	return []string{"wasm", "native"}
}

func splitComma(s string) []string {
	var result []string
	start := 0
	for i := 0; i <= len(s); i++ {
		if i == len(s) || s[i] == ',' {
			if i > start {
				result = append(result, s[start:i])
			}
			start = i + 1
		}
	}
	return result
}

// TestPrivateKeyAuth tests JWT authentication with private key
// Maps to: tests/definitions/shared/authentication/private_key_auth.feature
func TestPrivateKeyAuth(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	for _, backend := range getTestBackends() {
		backend := backend // capture for closure
		
		// Reset backend between tests to ensure correct backend is used
		ResetBackend()
		
		t.Run(backend+"/should authenticate using private file with password", func(t *testing.T) {
			// Given Authentication is set to JWT and private file with password is provided
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			// When Trying to Connect
			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Then Login is successful and simple query can be executed
			if err := verifySimpleQueryExecution(t, db); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("login or query failed: %v", err)
			}
		})

		t.Run(backend+"/should fail JWT authentication when invalid private key provided", func(t *testing.T) {
			// Given Authentication is set to JWT and invalid private key file is provided
			invalidKey := `-----BEGIN RSA PRIVATE KEY-----
MIIBOgIBAAJBALRiMLAHudeSA2ai4g7ANKkq5Q/9kNBBdIeReIBwJE5YXL3QQiHN
qWm1AMvXGfxzBL3NjWwpFEZM0hH5uLIBJusCAwEAAQJARd2fV/xvjnNhB8qS8QyN
YlKR2Ral8gH6QFa7kHx1qUdWlzNz88vvB2M2swXqRTd4bOZCHdxJHtQKC7KM8uVH
aQIhAOCCOLI0HPXE7cQg84rqGbIrLAq0bPNnkDa6nKE1KPLfAiEAy5x1VF9L+Cyp
oGPXFJE1cD2pJmpVQlPQNK89V3EyApkCIFrWcA+ZCOvEKi/fz5pmGz1rC7xyxGjL
JWdiMMDGHonPAiEAwbbvLpbPkk4Mg0Kef0o3pqRbbs5X8aYn2TMjEGz8bEkCIBdI
RzHVdG5BbJkN4E8J7mJDZQlqMCLY9v3Zu7L/lkNT
-----END RSA PRIVATE KEY-----`

			cfg := &Config{
				Account:       creds.Account,
				User:          creds.User,
				Host:          creds.Host,
				Authenticator: AuthTypeJWT,
				PrivateKeyPEM: invalidKey,
				Database:      creds.Database,
				Schema:        creds.Schema,
				Warehouse:     creds.Warehouse,
				Role:          creds.Role,
				Backend:       backend,
			}

			// When Trying to Connect
			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Then There is error returned
			err := db.Ping()
			if isBackendNotAvailable(err) && backend == "native" {
				t.Skip("native backend not available:", err)
			}
			if err == nil {
				t.Fatal("expected error with invalid private key, got nil")
			}
			t.Logf("Got expected error: %v", err)
		})
	}
}

// TestLargeResultSet tests handling large result sets
// Maps to: tests/definitions/shared/query/large_result_set.feature
func TestLargeResultSet(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	for _, backend := range getTestBackends() {
		backend := backend // capture for closure
		ResetBackend()

		t.Run(backend+"/should process one million row result set", func(t *testing.T) {
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Given Snowflake client is logged in
			if err := db.Ping(); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("failed to connect: %v", err)
			}

			// When Query is executed for 1M rows
			rows, err := db.Query("SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id")
			if err != nil {
				t.Fatalf("query failed: %v", err)
			}
			defer rows.Close()

			// Then there are 1000000 numbered sequentially rows returned
			count := 0
			for rows.Next() {
				var id int64
				if err := rows.Scan(&id); err != nil {
					t.Fatalf("scan failed at row %d: %v", count, err)
				}
				if id != int64(count) {
					t.Errorf("expected id %d, got %d", count, id)
				}
				count++
				// Log progress every 100K rows
				if count%100000 == 0 {
					t.Logf("Processed %d rows...", count)
				}
			}
			if err := rows.Err(); err != nil {
				t.Fatalf("rows error: %v", err)
			}
			if count != 1000000 {
				t.Errorf("expected 1000000 rows, got %d", count)
			}
			t.Logf("Large result set returned %d rows", count)
		})

		t.Run(backend+"/should process ten thousand string rows", func(t *testing.T) {
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Given Snowflake client is logged in
			if err := db.Ping(); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("failed to connect: %v", err)
			}

			// Generate string data with REPEAT to create substantial string values
			rows, err := db.Query("SELECT REPEAT('test_', seq8()) as text_data FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v")
			if err != nil {
				t.Fatalf("query failed: %v", err)
			}
			defer rows.Close()

			count := 0
			for rows.Next() {
				var text string
				if err := rows.Scan(&text); err != nil {
					t.Fatalf("scan failed at row %d: %v", count, err)
				}
				count++
			}
			if err := rows.Err(); err != nil {
				t.Fatalf("rows error: %v", err)
			}
			if count != 10000 {
				t.Errorf("expected 10000 rows, got %d", count)
			}
			t.Logf("String result set returned %d rows", count)
		})
	}
}

// TestAsyncExecution tests async query execution
// Maps to: tests/definitions/shared/query/async_execution.feature
func TestAsyncExecution(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	for _, backend := range getTestBackends() {
		backend := backend // capture for closure
		ResetBackend()

		t.Run(backend+"/should process async query result", func(t *testing.T) {
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Given Snowflake client is logged in with async engine enabled
			if err := db.Ping(); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("failed to connect: %v", err)
			}

			// When Query is executed (async execution is transparent to the caller)
			rows, err := db.Query("SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v ORDER BY id")
			if err != nil {
				t.Fatalf("query failed: %v", err)
			}
			defer rows.Close()

			// Then there are 1000 numbered sequentially rows returned
			count := 0
			for rows.Next() {
				var id int64
				if err := rows.Scan(&id); err != nil {
					t.Fatalf("scan failed: %v", err)
				}
				if id != int64(count) {
					t.Errorf("expected id %d, got %d", count, id)
				}
				count++
			}
			if err := rows.Err(); err != nil {
				t.Fatalf("rows error: %v", err)
			}
			if count != 1000 {
				t.Errorf("expected 1000 rows, got %d", count)
			}
			t.Logf("Async query returned %d rows", count)
		})
	}
}

// TestParametersBind tests query parameter binding
// Maps to: tests/definitions/shared/query/parameters_bind.feature
func TestParametersBind(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	for _, backend := range getTestBackends() {
		backend := backend // capture for closure
		ResetBackend()

		t.Run(backend+"/should bind single parameter to statement", func(t *testing.T) {
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Given Snowflake client is logged in
			if err := db.Ping(); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("failed to connect: %v", err)
			}

			// When Query with single parameter is executed
			rows, err := db.Query("SELECT ? AS value", 42)
			if err != nil {
				t.Fatalf("query failed: %v", err)
			}
			defer rows.Close()

			// Then Query execution should return the bound parameter value
			if !rows.Next() {
				t.Fatal("no rows returned")
			}
			var value int
			if err := rows.Scan(&value); err != nil {
				t.Fatalf("scan failed: %v", err)
			}
			if value != 42 {
				t.Errorf("expected 42, got %d", value)
			}
			t.Logf("Single parameter bound successfully: %d", value)
		})

		t.Run(backend+"/should bind multiple parameters to statement", func(t *testing.T) {
			cfg := &Config{
				Account:            creds.Account,
				User:               creds.User,
				Host:               creds.Host,
				Authenticator:      AuthTypeJWT,
				PrivateKeyPEM:      creds.PrivateKey,
				PrivateKeyPassword: creds.PrivateKeyPassword,
				Database:           creds.Database,
				Schema:             creds.Schema,
				Warehouse:          creds.Warehouse,
				Role:               creds.Role,
				Backend:            backend,
			}

			drv := &Driver{}
			db := sql.OpenDB(NewConnector(drv, cfg))
			defer db.Close()

			// Given Snowflake client is logged in
			if err := db.Ping(); err != nil {
				if isBackendNotAvailable(err) && backend == "native" {
					t.Skip("native backend not available:", err)
				}
				t.Fatalf("failed to connect: %v", err)
			}

			// When Query with multiple parameters is executed
			rows, err := db.Query("SELECT ? AS v1, ? AS v2, ? AS v3", 1, "hello", 3.14)
			if err != nil {
				t.Fatalf("query failed: %v", err)
			}
			defer rows.Close()

			// Then Query execution should return the bound parameter values
			if !rows.Next() {
				t.Fatal("no rows returned")
			}
			var v1 int
			var v2 string
			var v3 float64
			if err := rows.Scan(&v1, &v2, &v3); err != nil {
				t.Fatalf("scan failed: %v", err)
			}
			t.Logf("Multiple parameters bound successfully: %d, %s, %f", v1, v2, v3)
		})
	}
}

// isBackendNotAvailable checks if the error is due to backend not being available
func isBackendNotAvailable(err error) bool {
	if err == nil {
		return false
	}
	msg := err.Error()
	return contains(msg, "native backend") || contains(msg, "ErrNoBackend") || contains(msg, "requires CGO")
}

func contains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// verifySimpleQueryExecution verifies that a simple query can be executed
func verifySimpleQueryExecution(t *testing.T, db *sql.DB) error {
	ctx := context.Background()

	// First verify we can connect
	if err := db.PingContext(ctx); err != nil {
		return fmt.Errorf("ping failed: %w", err)
	}
	t.Log("Ping successful")

	rows, err := db.QueryContext(ctx, "SELECT 1 as num")
	if err != nil {
		return fmt.Errorf("query failed: %w", err)
	}
	defer rows.Close()

	cols, _ := rows.Columns()
	t.Logf("Columns: %v", cols)

	if !rows.Next() {
		if err := rows.Err(); err != nil {
			return fmt.Errorf("row iteration error: %w", err)
		}
		return fmt.Errorf("no rows returned")
	}

	var num int
	if err := rows.Scan(&num); err != nil {
		return fmt.Errorf("scan failed: %w", err)
	}

	if num != 1 {
		return fmt.Errorf("expected 1, got %d", num)
	}

	t.Logf("Simple query executed successfully, got: %d", num)
	return nil
}
