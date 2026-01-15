package snowflake

import (
	"crypto/rsa"
	"fmt"
	"time"
)

// AuthType represents the authentication method
type AuthType int

const (
	// AuthTypeSnowflake is the default password-based authentication
	AuthTypeSnowflake AuthType = iota
	// AuthTypeJWT is key pair authentication using JWT tokens
	AuthTypeJWT
	// AuthTypeExternalBrowser is browser-based SSO authentication
	AuthTypeExternalBrowser
	// AuthTypeOAuth is OAuth token authentication
	AuthTypeOAuth
	// AuthTypeOkta is Okta-based authentication
	AuthTypeOkta
	// AuthTypePAT is Personal Access Token authentication
	AuthTypePAT
)

// String returns the string representation of the auth type
func (a AuthType) String() string {
	switch a {
	case AuthTypeSnowflake:
		return "SNOWFLAKE"
	case AuthTypeJWT:
		return "SNOWFLAKE_JWT"
	case AuthTypeExternalBrowser:
		return "EXTERNALBROWSER"
	case AuthTypeOAuth:
		return "OAUTH"
	case AuthTypeOkta:
		return "OKTA"
	case AuthTypePAT:
		return "PAT"
	default:
		return "SNOWFLAKE"
	}
}

// ParseAuthType converts a string to AuthType
func ParseAuthType(s string) AuthType {
	switch s {
	case "SNOWFLAKE":
		return AuthTypeSnowflake
	case "SNOWFLAKE_JWT":
		return AuthTypeJWT
	case "EXTERNALBROWSER":
		return AuthTypeExternalBrowser
	case "OAUTH":
		return AuthTypeOAuth
	case "OKTA":
		return AuthTypeOkta
	case "PAT":
		return AuthTypePAT
	default:
		return AuthTypeSnowflake
	}
}

// Config is the configuration for a Snowflake connection
type Config struct {
	// Account is the Snowflake account identifier
	Account string
	// User is the login user name
	User string
	// Password is the login password (for password-based auth)
	Password string
	// Database is the default database
	Database string
	// Schema is the default schema
	Schema string
	// Warehouse is the default warehouse
	Warehouse string
	// Role is the default role
	Role string

	// Host is the Snowflake host (optional, derived from account)
	Host string
	// Port is the Snowflake port (default: 443)
	Port int
	// Protocol is the connection protocol (default: https)
	Protocol string

	// Authenticator is the authentication method
	Authenticator AuthType

	// Token is the OAuth or PAT token
	Token string

	// PrivateKey is the RSA private key for JWT auth (unencrypted/decrypted)
	PrivateKey *rsa.PrivateKey
	// PrivateKeyPEM is the raw PEM-encoded private key (may be encrypted)
	// If set, this is passed to the backend which handles decryption
	PrivateKeyPEM string
	// PrivateKeyPath is the path to the private key file
	PrivateKeyPath string
	// PrivateKeyPassword is the password for encrypted private key
	PrivateKeyPassword string

	// Params contains additional connection parameters
	Params map[string]string

	// Timeouts
	LoginTimeout   time.Duration
	RequestTimeout time.Duration
	
	// Backend selects the backend implementation ("wasm" or "native")
	Backend string
}

// Validate validates the configuration
func (c *Config) Validate() error {
	if c.Account == "" {
		return ErrEmptyAccount
	}
	if c.User == "" {
		return ErrEmptyUser
	}
	return nil
}

// HostWithPort returns the host with port
func (c *Config) HostWithPort() string {
	if c.Host == "" {
		return ""
	}
	if c.Port == 0 || c.Port == 443 {
		return c.Host
	}
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

// DefaultConfig returns a Config with default values
func DefaultConfig() *Config {
	return &Config{
		Port:           443,
		Protocol:       "https",
		Authenticator:  AuthTypeSnowflake,
		LoginTimeout:   300 * time.Second,
		RequestTimeout: 0,
		Params:         make(map[string]string),
		Backend:        "wasm", // Default to WASM backend
	}
}

