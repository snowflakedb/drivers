# TLS Configuration for Thrift API

## Overview

The `TlsConfig` provides a simple, unified way to configure TLS settings including certificate revocation checking via CRLs. This document outlines how to expose this through Thrift APIs for language-specific clients.

The system is fundamentally a **TLS client** that supports certificate revocation checking as one of its security features, rather than being primarily a CRL tool. CRL checks are enforced during the TLS handshake via a custom rustls `ServerCertVerifier`.

## Thrift IDL Design

```thrift
// TLS Certificate Revocation Check Mode
enum CertRevocationCheckMode {
    DISABLED = 0,  // No CRL checking (default)
    ENABLED = 1,   // Fail connection if certificate is revoked or CRL check fails
    ADVISORY = 2   // Log warnings but allow connection if CRL check fails
}

// TLS Configuration
struct TlsConfig {
    // Certificate Revocation List settings
    1: optional CertRevocationCheckMode crl_mode = CertRevocationCheckMode.DISABLED,
    2: optional bool crl_disk_caching = true,
    3: optional bool crl_memory_caching = true,
    4: optional string crl_cache_dir,
    5: optional i32 crl_validity_days = 10,
    6: optional bool allow_certs_without_crl_url = false,
    7: optional i32 crl_http_timeout_seconds = 30,
    8: optional i32 crl_connection_timeout_seconds = 10,
    
    // Root certificate store
    9: optional string custom_root_store_path,  // Path to PEM file with custom root certificates
    
    // General TLS settings
    10: optional bool verify_hostname = true,
    11: optional bool verify_certificates = true,
}

// Connection parameters that include TLS config
struct ConnectionParams {
    1: required string server_url,
    2: optional string username,
    3: optional string password,
    4: optional TlsConfig tls_config,
    // ... other connection parameters
}
```

## Language-Specific Client Usage

### Python Client
```python
from snowflake_driver import TlsConfig, CertRevocationCheckMode, connect

# Production configuration
tls_config = TlsConfig(
    crl_mode=CertRevocationCheckMode.ENABLED,
    custom_root_store_path="/path/to/enterprise/roots.pem"
)

conn = connect(
    server_url="https://account.snowflakecomputing.com",
    username="user",
    password="pass",
    tls_config=tls_config
)
```

### Java Client
```java
import com.snowflake.driver.TlsConfig;
import com.snowflake.driver.CertRevocationCheckMode;

TlsConfig tlsConfig = new TlsConfig()
    .setCrlMode(CertRevocationCheckMode.ENABLED)
    .setCustomRootStorePath("/path/to/enterprise/roots.pem")
    .setCrlDiskCaching(true);

Connection conn = DriverManager.getConnection(
    "jdbc:snowflake://account.snowflakecomputing.com",
    props,
    tlsConfig
);
```

### Go Client
```go
import "github.com/snowflake/go-driver"

tlsConfig := &snowflake.TlsConfig{
    CrlMode: snowflake.CertRevocationCheckModeEnabled,
    CustomRootStorePath: "/path/to/enterprise/roots.pem",
    CrlDiskCaching: true,
}

conn, err := snowflake.Connect(ctx, &snowflake.ConnectionParams{
    ServerURL: "https://account.snowflakecomputing.com",
    Username:  "user", 
    Password:  "pass",
    TlsConfig: tlsConfig,
})
```

### Node.js Client
```javascript
const { connect, TlsConfig, CertRevocationCheckMode } = require('snowflake-sdk');

const tlsConfig = new TlsConfig({
    crlMode: CertRevocationCheckMode.ENABLED,
    customRootStorePath: '/path/to/enterprise/roots.pem',
    crlDiskCaching: true
});

const connection = await connect({
    serverUrl: 'https://account.snowflakecomputing.com',
    username: 'user',
    password: 'pass',
    tlsConfig: tlsConfig
});
```

## Rust Implementation Mapping

The Thrift `TlsConfig` maps directly to our Rust `TlsConfig`:

```rust
// Convert from Thrift TlsConfig to Rust TlsConfig
impl From<thrift::TlsConfig> for sf_core::tls::TlsConfig {
    fn from(thrift_config: thrift::TlsConfig) -> Self {
        let crl_config = sf_core::crl::CrlConfig {
            check_mode: thrift_config.crl_mode.unwrap_or_default().into(),
            enable_disk_caching: thrift_config.crl_disk_caching.unwrap_or(true),
            enable_memory_caching: thrift_config.crl_memory_caching.unwrap_or(true),
            cache_dir: thrift_config.crl_cache_dir.map(PathBuf::from),
            validity_time: Duration::days(thrift_config.crl_validity_days.unwrap_or(10) as i64),
            allow_certificates_without_crl_url: thrift_config.allow_certs_without_crl_url.unwrap_or(false),
            http_timeout: Duration::seconds(thrift_config.crl_http_timeout_seconds.unwrap_or(30) as i64),
            connection_timeout: Duration::seconds(thrift_config.crl_connection_timeout_seconds.unwrap_or(10) as i64),
        };
        
        sf_core::tls::TlsConfig {
            crl_config,
            custom_root_store_path: thrift_config.custom_root_store_path.map(PathBuf::from),
            verify_hostname: thrift_config.verify_hostname.unwrap_or(true),
            verify_certificates: thrift_config.verify_certificates.unwrap_or(true),
        }
    }
}
```

## Benefits of This Design

1. **Simple for Callers**: One config object handles all TLS settings
2. **Language Agnostic**: Thrift generates bindings for all target languages
3. **Backwards Compatible**: All fields are optional with sensible defaults
4. **Enterprise Ready**: Supports custom root stores for corporate environments
5. **Flexible**: Can disable individual validation components for testing
6. **Future Proof**: Easy to add new TLS settings without breaking existing clients

## Default Configurations

- **Production**: CRL enabled, all verification enabled
- **Development**: CRL advisory mode, all verification enabled  
- **Testing**: CRL disabled, certificate verification enabled
- **Insecure**: All validation disabled (for testing only)

## Security Considerations

- Default is secure (CRL disabled but certificates verified)
- Insecure options require explicit configuration
- Custom root stores are loaded from file paths (not embedded in config)
- CRL validation adds latency but improves security
- Advisory mode provides logging without breaking connections
