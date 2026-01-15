package snowflake

import (
	"context"
	"database/sql"
	"database/sql/driver"
)

func init() {
	sql.Register("snowflake", &Driver{})
}

// Driver is the Snowflake driver implementing database/sql/driver.Driver
type Driver struct{}

// Open opens a new database connection
func (d *Driver) Open(dsn string) (driver.Conn, error) {
	cfg, err := ParseDSN(dsn)
	if err != nil {
		return nil, err
	}
	return d.OpenWithConfig(context.Background(), cfg)
}

// OpenConnector returns a new connector for the given DSN
func (d *Driver) OpenConnector(dsn string) (driver.Connector, error) {
	cfg, err := ParseDSN(dsn)
	if err != nil {
		return nil, err
	}
	return NewConnector(d, cfg), nil
}

// OpenWithConfig opens a new database connection with the given configuration
func (d *Driver) OpenWithConfig(ctx context.Context, cfg *Config) (driver.Conn, error) {
	// Get or create backend
	backend, err := GetBackend(ctx, cfg.Backend)
	if err != nil {
		return nil, &ConnectionError{Message: "failed to initialize backend", Cause: err}
	}

	// Create database handle
	dbHandle, err := backend.DatabaseNew(ctx)
	if err != nil {
		return nil, &ConnectionError{Message: "failed to create database", Cause: err}
	}

	// Initialize database
	if err := backend.DatabaseInit(ctx, dbHandle); err != nil {
		return nil, &ConnectionError{Message: "failed to initialize database", Cause: err}
	}

	// Create connection handle
	connHandle, err := backend.ConnectionNew(ctx)
	if err != nil {
		return nil, &ConnectionError{Message: "failed to create connection", Cause: err}
	}

	// Set connection options from config
	if err := setConnectionOptionsImpl(ctx, backend, connHandle, cfg); err != nil {
		return nil, err
	}

	// Initialize connection (performs login)
	if err := backend.ConnectionInit(ctx, connHandle, dbHandle); err != nil {
		return nil, &AuthError{Message: "login failed", Cause: err}
	}

	return &Conn{
		backend:    backend,
		dbHandle:   dbHandle,
		connHandle: connHandle,
		cfg:        cfg,
	}, nil
}


// Connector implements database/sql/driver.Connector
type Connector struct {
	driver *Driver
	cfg    *Config
}

// NewConnector creates a new Connector with the given config
func NewConnector(driver *Driver, cfg *Config) *Connector {
	return &Connector{
		driver: driver,
		cfg:    cfg,
	}
}

// Connect implements driver.Connector
func (c *Connector) Connect(ctx context.Context) (driver.Conn, error) {
	return c.driver.OpenWithConfig(ctx, c.cfg)
}

// Driver implements driver.Connector
func (c *Connector) Driver() driver.Driver {
	return c.driver
}
