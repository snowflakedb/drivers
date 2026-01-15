package snowflake

import (
	"database/sql"
	"encoding/json"
	"os"
	"strings"
	"testing"
)

// TestCredentials holds test connection credentials
type TestCredentials struct {
	User               string
	Password           string
	Account            string
	Role               string
	Warehouse          string
	Database           string
	Schema             string
	Host               string
	PrivateKey         string
	PrivateKeyPassword string
}

// loadTestCredentials loads credentials from parameters.json
func loadTestCredentials() (*TestCredentials, error) {
	// Try common locations
	paths := []string{
		os.Getenv("SNOWFLAKE_PARAMETERS_FILE"),
		"/Users/snoonan/parameters.json",
		"../../parameters.json",
		"../../../parameters.json",
	}

	var data []byte
	var err error
	for _, path := range paths {
		if path == "" {
			continue
		}
		data, err = os.ReadFile(path)
		if err == nil {
			break
		}
	}
	if data == nil {
		return nil, err
	}

	var params struct {
		TestConnection struct {
			User               string   `json:"SNOWFLAKE_TEST_USER"`
			Password           string   `json:"SNOWFLAKE_TEST_PASSWORD"`
			Account            string   `json:"SNOWFLAKE_TEST_ACCOUNT"`
			Role               string   `json:"SNOWFLAKE_TEST_ROLE"`
			Warehouse          string   `json:"SNOWFLAKE_TEST_WAREHOUSE"`
			Database           string   `json:"SNOWFLAKE_TEST_DATABASE"`
			Schema             string   `json:"SNOWFLAKE_TEST_SCHEMA"`
			Host               string   `json:"SNOWFLAKE_TEST_HOST"`
			PrivateKeyPassword string   `json:"SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD"`
			PrivateKeyContents []string `json:"SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS"`
		} `json:"testconnection"`
	}

	if err := json.Unmarshal(data, &params); err != nil {
		return nil, err
	}

	return &TestCredentials{
		User:               params.TestConnection.User,
		Password:           params.TestConnection.Password,
		Account:            params.TestConnection.Account,
		Role:               params.TestConnection.Role,
		Warehouse:          params.TestConnection.Warehouse,
		Database:           params.TestConnection.Database,
		Schema:             params.TestConnection.Schema,
		Host:               params.TestConnection.Host,
		PrivateKey:         strings.Join(params.TestConnection.PrivateKeyContents, "\n"),
		PrivateKeyPassword: params.TestConnection.PrivateKeyPassword,
	}, nil
}

func TestParseDSN(t *testing.T) {
	tests := []struct {
		name    string
		dsn     string
		wantErr bool
		check   func(*Config) error
	}{
		{
			name: "URL style - simple",
			dsn:  "user@account",
			check: func(cfg *Config) error {
				if cfg.User != "user" {
					t.Errorf("expected user 'user', got '%s'", cfg.User)
				}
				if cfg.Account != "account" {
					t.Errorf("expected account 'account', got '%s'", cfg.Account)
				}
				return nil
			},
		},
		{
			name: "URL style - with password",
			dsn:  "user:password@account",
			check: func(cfg *Config) error {
				if cfg.User != "user" {
					t.Errorf("expected user 'user', got '%s'", cfg.User)
				}
				if cfg.Password != "password" {
					t.Errorf("expected password 'password', got '%s'", cfg.Password)
				}
				return nil
			},
		},
		{
			name: "URL style - with database/schema",
			dsn:  "user@account/mydb/myschema",
			check: func(cfg *Config) error {
				if cfg.Database != "mydb" {
					t.Errorf("expected database 'mydb', got '%s'", cfg.Database)
				}
				if cfg.Schema != "myschema" {
					t.Errorf("expected schema 'myschema', got '%s'", cfg.Schema)
				}
				return nil
			},
		},
		{
			name: "URL style - with params",
			dsn:  "user@account?warehouse=wh&role=role1",
			check: func(cfg *Config) error {
				if cfg.Warehouse != "wh" {
					t.Errorf("expected warehouse 'wh', got '%s'", cfg.Warehouse)
				}
				if cfg.Role != "role1" {
					t.Errorf("expected role 'role1', got '%s'", cfg.Role)
				}
				return nil
			},
		},
		{
			name: "Key-value style",
			dsn:  "account=myaccount&user=myuser&password=mypass",
			check: func(cfg *Config) error {
				if cfg.Account != "myaccount" {
					t.Errorf("expected account 'myaccount', got '%s'", cfg.Account)
				}
				if cfg.User != "myuser" {
					t.Errorf("expected user 'myuser', got '%s'", cfg.User)
				}
				if cfg.Password != "mypass" {
					t.Errorf("expected password 'mypass', got '%s'", cfg.Password)
				}
				return nil
			},
		},
		{
			name:    "Missing account",
			dsn:     "user=myuser",
			wantErr: true,
		},
		{
			name:    "Missing user",
			dsn:     "account=myaccount",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg, err := ParseDSN(tt.dsn)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseDSN() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				if err := tt.check(cfg); err != nil {
					t.Error(err)
				}
			}
		})
	}
}

func TestDriverOpen(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	// Build DSN with password auth (simpler for DSN test)
	dsn := "account=" + creds.Account +
		"&user=" + creds.User +
		"&host=" + creds.Host +
		"&database=" + creds.Database +
		"&schema=" + creds.Schema +
		"&warehouse=" + creds.Warehouse +
		"&role=" + creds.Role +
		"&backend=wasm"

	t.Logf("Connecting with DSN: %s", dsn)

	// sql.Open doesn't actually connect, just parses DSN
	db, err := sql.Open("snowflake", dsn)
	if err != nil {
		t.Fatalf("failed to parse DSN: %v", err)
	}
	defer db.Close()

	t.Log("DSN parsed successfully (connection requires explicit Ping)")
}

func TestSimpleQuery(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	creds, err := loadTestCredentials()
	if err != nil {
		t.Skip("credentials not available:", err)
	}

	cfg := &Config{
		Account:            creds.Account,
		User:               creds.User,
		Host:               creds.Host,
		Authenticator:      AuthTypeJWT,
		PrivateKeyPEM:      creds.PrivateKey,  // Raw PEM, let backend handle decryption
		PrivateKeyPassword: creds.PrivateKeyPassword,
		Database:           creds.Database,
		Schema:             creds.Schema,
		Warehouse:          creds.Warehouse,
		Role:               creds.Role,
		Backend:            "wasm",
	}

	drv := &Driver{}
	conn, err := drv.OpenWithConfig(t.Context(), cfg)
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	defer conn.Close()

	// Use database/sql wrapper for better interface
	db := sql.OpenDB(NewConnector(drv, cfg))
	defer db.Close()

	// Execute query
	rows, err := db.Query("SELECT 1 as num, 'Hello' as message")
	if err != nil {
		t.Fatalf("failed to execute query: %v", err)
	}
	defer rows.Close()

	// Fetch results
	cols, _ := rows.Columns()
	t.Logf("Columns: %v", cols)

	for rows.Next() {
		var num int
		var message string
		if err := rows.Scan(&num, &message); err != nil {
			t.Fatalf("failed to scan row: %v", err)
		}
		t.Logf("Row: num=%d, message=%s", num, message)
	}

	if err := rows.Err(); err != nil {
		t.Fatalf("error during iteration: %v", err)
	}
}
