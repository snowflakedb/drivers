package snowflake

import (
	"context"
	"crypto/x509"
	"database/sql/driver"
	"encoding/pem"

	pb "github.com/snowflakedb/universal-driver/go/protobuf"
)

// Conn implements database/sql/driver.Conn
type Conn struct {
	backend    Backend
	dbHandle   *pb.DatabaseHandle
	connHandle *pb.ConnectionHandle
	cfg        *Config
	closed     bool
}

var (
	_ driver.Conn               = (*Conn)(nil)
	_ driver.ConnPrepareContext = (*Conn)(nil)
	_ driver.ConnBeginTx        = (*Conn)(nil)
	_ driver.Pinger             = (*Conn)(nil)
	_ driver.SessionResetter    = (*Conn)(nil)
)

// Prepare implements driver.Conn
func (c *Conn) Prepare(query string) (driver.Stmt, error) {
	return c.PrepareContext(context.Background(), query)
}

// PrepareContext implements driver.ConnPrepareContext
func (c *Conn) PrepareContext(ctx context.Context, query string) (driver.Stmt, error) {
	if c.closed {
		return nil, ErrConnectionClosed
	}

	stmtHandle, err := c.backend.StatementNew(ctx, c.connHandle)
	if err != nil {
		return nil, err
	}

	if err := c.backend.StatementSetSqlQuery(ctx, stmtHandle, query); err != nil {
		c.backend.StatementRelease(ctx, stmtHandle)
		return nil, err
	}

	return &Stmt{
		conn:       c,
		stmtHandle: stmtHandle,
		query:      query,
	}, nil
}

// Close implements driver.Conn
func (c *Conn) Close() error {
	if c.closed {
		return nil
	}
	c.closed = true

	ctx := context.Background()

	if c.connHandle != nil {
		c.backend.ConnectionRelease(ctx, c.connHandle)
	}
	if c.dbHandle != nil {
		c.backend.DatabaseRelease(ctx, c.dbHandle)
	}

	return nil
}

// Begin implements driver.Conn (deprecated, use BeginTx)
func (c *Conn) Begin() (driver.Tx, error) {
	return c.BeginTx(context.Background(), driver.TxOptions{})
}

// BeginTx implements driver.ConnBeginTx
func (c *Conn) BeginTx(ctx context.Context, opts driver.TxOptions) (driver.Tx, error) {
	if c.closed {
		return nil, ErrConnectionClosed
	}
	// Snowflake auto-commits by default, but we can implement explicit transactions
	return &Tx{conn: c}, nil
}

// Ping implements driver.Pinger
func (c *Conn) Ping(ctx context.Context) error {
	if c.closed {
		return ErrConnectionClosed
	}
	// Execute a simple query to verify connection
	stmt, err := c.PrepareContext(ctx, "SELECT 1")
	if err != nil {
		return err
	}
	defer stmt.Close()

	rows, err := stmt.(*Stmt).QueryContext(ctx, nil)
	if err != nil {
		return err
	}
	defer rows.Close()

	return nil
}

// ResetSession implements driver.SessionResetter
func (c *Conn) ResetSession(ctx context.Context) error {
	if c.closed {
		return ErrConnectionClosed
	}
	// Connection is stateless, nothing to reset
	return nil
}

// ExecContext executes a query without returning rows
func (c *Conn) ExecContext(ctx context.Context, query string, args []driver.NamedValue) (driver.Result, error) {
	stmt, err := c.PrepareContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	return stmt.(*Stmt).ExecContext(ctx, args)
}

// QueryContext executes a query that returns rows
func (c *Conn) QueryContext(ctx context.Context, query string, args []driver.NamedValue) (driver.Rows, error) {
	stmt, err := c.PrepareContext(ctx, query)
	if err != nil {
		return nil, err
	}
	// Note: don't defer stmt.Close() here - rows owns the statement

	return stmt.(*Stmt).QueryContext(ctx, args)
}

// Tx implements database/sql/driver.Tx
type Tx struct {
	conn *Conn
}

// Commit implements driver.Tx
func (t *Tx) Commit() error {
	// Snowflake auto-commits, this is a no-op
	return nil
}

// Rollback implements driver.Tx
func (t *Tx) Rollback() error {
	// Snowflake auto-commits, rollback is a no-op
	return nil
}

// setConnectionOptionsImpl sets all connection options from the config
func setConnectionOptionsImpl(ctx context.Context, backend Backend, connHandle *pb.ConnectionHandle, cfg *Config) error {
	ch := connHandle

	// Required options
	if cfg.Host != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "host", cfg.Host); err != nil {
			return err
		}
	}
	if cfg.Account != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "account", cfg.Account); err != nil {
			return err
		}
	}
	if cfg.User != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "user", cfg.User); err != nil {
			return err
		}
	}

	// Authentication
	if err := backend.ConnectionSetOptionString(ctx, ch, "authenticator", cfg.Authenticator.String()); err != nil {
		return err
	}

	// Password or JWT auth
	if cfg.Password != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "password", cfg.Password); err != nil {
			return err
		}
	}

	// Private key for JWT auth
	if cfg.PrivateKeyPEM != "" {
		// Use raw PEM (may be encrypted, backend will handle decryption)
		if err := backend.ConnectionSetOptionString(ctx, ch, "private_key", cfg.PrivateKeyPEM); err != nil {
			return err
		}
	} else if cfg.PrivateKey != nil {
		// Serialize already-decrypted private key to PEM
		keyBytes := x509.MarshalPKCS1PrivateKey(cfg.PrivateKey)
		pemBlock := &pem.Block{
			Type:  "RSA PRIVATE KEY",
			Bytes: keyBytes,
		}
		pemBytes := pem.EncodeToMemory(pemBlock)
		if err := backend.ConnectionSetOptionString(ctx, ch, "private_key", string(pemBytes)); err != nil {
			return err
		}
	}

	if cfg.PrivateKeyPassword != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "private_key_password", cfg.PrivateKeyPassword); err != nil {
			return err
		}
	}

	// Token for OAuth/PAT auth
	if cfg.Token != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "token", cfg.Token); err != nil {
			return err
		}
	}

	// Optional options
	if cfg.Database != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "database", cfg.Database); err != nil {
			return err
		}
	}
	if cfg.Schema != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "schema", cfg.Schema); err != nil {
			return err
		}
	}
	if cfg.Warehouse != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "warehouse", cfg.Warehouse); err != nil {
			return err
		}
	}
	if cfg.Role != "" {
		if err := backend.ConnectionSetOptionString(ctx, ch, "role", cfg.Role); err != nil {
			return err
		}
	}

	// Additional params
	for k, v := range cfg.Params {
		if err := backend.ConnectionSetOptionString(ctx, ch, k, v); err != nil {
			return err
		}
	}

	return nil
}
