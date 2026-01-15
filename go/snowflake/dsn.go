package snowflake

import (
	"crypto/rsa"
	"crypto/x509"
	"encoding/pem"
	"fmt"
	"net/url"
	"os"
	"strconv"
	"strings"
)

const (
	defaultDomain = ".snowflakecomputing.com"
)

// ParseDSN parses a DSN string into a Config
//
// DSN formats supported:
//   - user[:password]@account[/database[/schema]][?param1=value1&...]
//   - account=<account>&user=<user>&password=<password>&...
func ParseDSN(dsn string) (*Config, error) {
	cfg := DefaultConfig()

	// Check if it's URL-style (contains @) or key=value style
	if strings.Contains(dsn, "@") {
		return parseURLStyleDSN(dsn, cfg)
	}
	return parseKeyValueDSN(dsn, cfg)
}

// parseURLStyleDSN parses: user[:password]@account[/database[/schema]][?params]
func parseURLStyleDSN(dsn string, cfg *Config) (*Config, error) {
	// Split query params
	queryIdx := strings.Index(dsn, "?")
	var queryStr string
	if queryIdx >= 0 {
		queryStr = dsn[queryIdx+1:]
		dsn = dsn[:queryIdx]
	}

	// Split user info and host
	atIdx := strings.Index(dsn, "@")
	if atIdx < 0 {
		return nil, ErrInvalidDSN
	}

	userInfo := dsn[:atIdx]
	hostPath := dsn[atIdx+1:]

	// Parse user:password
	if colonIdx := strings.Index(userInfo, ":"); colonIdx >= 0 {
		cfg.User = userInfo[:colonIdx]
		cfg.Password = userInfo[colonIdx+1:]
	} else {
		cfg.User = userInfo
	}

	// Parse account/database/schema
	parts := strings.Split(hostPath, "/")
	if len(parts) >= 1 {
		cfg.Account = parts[0]
	}
	if len(parts) >= 2 {
		cfg.Database = parts[1]
	}
	if len(parts) >= 3 {
		cfg.Schema = parts[2]
	}

	// Parse query params
	if queryStr != "" {
		if err := parseQueryParams(queryStr, cfg); err != nil {
			return nil, err
		}
	}

	// Derive host from account if not specified
	if cfg.Host == "" && cfg.Account != "" {
		cfg.Host = deriveHost(cfg.Account)
	}

	return cfg, cfg.Validate()
}

// parseKeyValueDSN parses: account=<account>&user=<user>&...
func parseKeyValueDSN(dsn string, cfg *Config) (*Config, error) {
	if err := parseQueryParams(dsn, cfg); err != nil {
		return nil, err
	}

	// Derive host from account if not specified
	if cfg.Host == "" && cfg.Account != "" {
		cfg.Host = deriveHost(cfg.Account)
	}

	return cfg, cfg.Validate()
}

// parseQueryParams parses query parameters into config
func parseQueryParams(query string, cfg *Config) error {
	values, err := url.ParseQuery(query)
	if err != nil {
		return fmt.Errorf("invalid query string: %w", err)
	}

	for key, vals := range values {
		if len(vals) == 0 {
			continue
		}
		val := vals[0]

		switch strings.ToLower(key) {
		case "account":
			cfg.Account = val
		case "user":
			cfg.User = val
		case "password":
			cfg.Password = val
		case "database":
			cfg.Database = val
		case "schema":
			cfg.Schema = val
		case "warehouse":
			cfg.Warehouse = val
		case "role":
			cfg.Role = val
		case "host":
			cfg.Host = val
		case "port":
			if p, err := strconv.Atoi(val); err == nil {
				cfg.Port = p
			}
		case "protocol":
			cfg.Protocol = val
		case "authenticator":
			cfg.Authenticator = ParseAuthType(strings.ToUpper(val))
		case "token":
			cfg.Token = val
		case "private_key":
			key, err := parsePrivateKey([]byte(val), "")
			if err != nil {
				return fmt.Errorf("failed to parse private_key: %w", err)
			}
			cfg.PrivateKey = key
		case "private_key_file":
			cfg.PrivateKeyPath = val
		case "private_key_password":
			cfg.PrivateKeyPassword = val
		case "backend":
			cfg.Backend = val
		default:
			// Store unknown params
			if cfg.Params == nil {
				cfg.Params = make(map[string]string)
			}
			cfg.Params[key] = val
		}
	}

	// Load private key from file if specified
	if cfg.PrivateKeyPath != "" && cfg.PrivateKey == nil {
		key, err := LoadPrivateKeyFile(cfg.PrivateKeyPath, cfg.PrivateKeyPassword)
		if err != nil {
			return err
		}
		cfg.PrivateKey = key
	}

	return nil
}

// deriveHost derives the host from account name
func deriveHost(account string) string {
	// Check if account already contains a domain
	if strings.Contains(account, ".") {
		// Could be a full URL or region-qualified account
		if strings.Contains(account, defaultDomain) {
			return account
		}
		// Assume it's a region-qualified account like "account.region.cloud"
		return account + defaultDomain
	}
	// Simple account name
	return account + defaultDomain
}

// LoadPrivateKeyFile loads a private key from a file
func LoadPrivateKeyFile(path, password string) (*rsa.PrivateKey, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read private key file: %w", err)
	}
	return parsePrivateKey(data, password)
}

// parsePrivateKey parses a PEM-encoded private key
func parsePrivateKey(data []byte, password string) (*rsa.PrivateKey, error) {
	block, _ := pem.Decode(data)
	if block == nil {
		return nil, fmt.Errorf("failed to decode PEM block")
	}

	var keyData []byte
	var err error

	// Check if encrypted (PKCS#8 encrypted or PEM encrypted)
	if strings.Contains(block.Type, "ENCRYPTED") {
		if password == "" {
			return nil, fmt.Errorf("private key is encrypted but no password provided")
		}
		// Try PKCS#8 encrypted first
		key, err := decryptPKCS8(block.Bytes, []byte(password))
		if err == nil {
			rsaKey, ok := key.(*rsa.PrivateKey)
			if !ok {
				return nil, fmt.Errorf("private key is not RSA")
			}
			return rsaKey, nil
		}
		// Fall back to older PEM encryption
		keyData, err = x509.DecryptPEMBlock(block, []byte(password))
		if err != nil {
			return nil, fmt.Errorf("failed to decrypt private key: %w", err)
		}
	} else if x509.IsEncryptedPEMBlock(block) {
		if password == "" {
			return nil, fmt.Errorf("private key is encrypted but no password provided")
		}
		keyData, err = x509.DecryptPEMBlock(block, []byte(password))
		if err != nil {
			return nil, fmt.Errorf("failed to decrypt private key: %w", err)
		}
	} else {
		keyData = block.Bytes
	}

	// Try PKCS#8 first, then PKCS#1
	key, err := x509.ParsePKCS8PrivateKey(keyData)
	if err == nil {
		rsaKey, ok := key.(*rsa.PrivateKey)
		if !ok {
			return nil, fmt.Errorf("private key is not RSA")
		}
		return rsaKey, nil
	}

	rsaKey, err := x509.ParsePKCS1PrivateKey(keyData)
	if err != nil {
		return nil, fmt.Errorf("failed to parse private key: %w", err)
	}
	return rsaKey, nil
}

// decryptPKCS8 decrypts a PKCS#8 encrypted private key
func decryptPKCS8(data, password []byte) (interface{}, error) {
	// Try to parse as encrypted PKCS#8
	key, err := x509.DecryptPEMBlock(&pem.Block{
		Type:  "ENCRYPTED PRIVATE KEY",
		Bytes: data,
	}, password)
	if err != nil {
		// Not a PEM-encrypted block, try PKCS#8 encryption
		// This requires additional crypto handling
		return nil, err
	}
	return x509.ParsePKCS8PrivateKey(key)
}

// DSN builds a DSN string from a Config
func (c *Config) DSN() string {
	var buf strings.Builder

	// user:password@account
	buf.WriteString(c.User)
	if c.Password != "" {
		buf.WriteByte(':')
		buf.WriteString(c.Password)
	}
	buf.WriteByte('@')
	buf.WriteString(c.Account)

	// /database/schema
	if c.Database != "" {
		buf.WriteByte('/')
		buf.WriteString(c.Database)
		if c.Schema != "" {
			buf.WriteByte('/')
			buf.WriteString(c.Schema)
		}
	}

	// Query params
	params := url.Values{}
	if c.Warehouse != "" {
		params.Set("warehouse", c.Warehouse)
	}
	if c.Role != "" {
		params.Set("role", c.Role)
	}
	if c.Authenticator != AuthTypeSnowflake {
		params.Set("authenticator", c.Authenticator.String())
	}
	if c.Host != "" {
		params.Set("host", c.Host)
	}
	if c.Port != 443 && c.Port != 0 {
		params.Set("port", strconv.Itoa(c.Port))
	}
	if c.Backend != "" && c.Backend != "wasm" {
		params.Set("backend", c.Backend)
	}

	for k, v := range c.Params {
		params.Set(k, v)
	}

	if len(params) > 0 {
		buf.WriteByte('?')
		buf.WriteString(params.Encode())
	}

	return buf.String()
}
