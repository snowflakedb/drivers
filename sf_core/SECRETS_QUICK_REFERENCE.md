# Secrets Masking Quick Reference

## Quick Start

### 1. For Sensitive String Fields
```rust
use sf_core::secrets::SecretString;

// Define field as SecretString
pub struct Config {
    pub password: SecretString,
}

// Create from string
let password = SecretString::new("my_password".to_string());
let password = SecretString::from_str("my_password");

// Use in code
println!("{}", password);           // Shows: ****
let actual = password.expose_secret();  // Get actual value for API calls
```

### 2. For Logging Structures with Secrets
```rust
use crate::rest::snowflake::auth::AuthRequest;

// WRONG - DO NOT DO THIS:
tracing::debug!("Request: {:?}", auth_request);

// CORRECT - Use to_safe():
tracing::debug!("Request: {:?}", auth_request.to_safe());
```

### 3. For SQL Query Logging
```rust
use sf_core::secrets::redact_query;

let sql = "CREATE USER alice PASSWORD = 'secret'";
tracing::error!(sql = redact_query(sql), "Query failed");
```

### 4. For Dynamic Field Masking
```rust
use sf_core::secrets::mask_value;

let value = mask_value("password", Some("secret123"));  // Returns: "****"
let value = mask_value("username", Some("alice"));      // Returns: "alice"
```

## Sensitive Field Names (Auto-Masked)

These field names are automatically recognized as sensitive:

```
password, token, secret, api_key, private_key, auth, authorization,
credential, credentials, saml_response, raw_saml_response, proof_key,
master_token, session_token, access_token, refresh_token, jwt,
passcode, passphrase, oauth
```

## Pattern Detection (Auto-Masked)

These patterns are automatically identified as secrets:

- JWT tokens: `eyJhbG...` (3 base64 segments with dots)
- Bearer tokens: `Bearer abc123...`
- API key prefixes: `sk_`, `pk_`, `api_`, `token_`, `ghp_`, `github_pat_`
- High-entropy strings: Long mixed-case strings with numbers/symbols

## When to Use What

| Scenario | Use This | Example |
|----------|----------|---------|
| Struct field storing password/token | `SecretString` | `pub password: SecretString` |
| Logging auth structures | `.to_safe()` | `auth_request.to_safe()` |
| Logging SQL queries | `redact_query()` | `redact_query(sql)` |
| Dynamic field masking | `mask_value()` | `mask_value("password", val)` |
| Partial visibility needed | `mask_partial()` | `mask_partial(session_id, 3)` |

## Common Mistakes to Avoid

### ❌ DON'T
```rust
// Don't log raw sensitive structures
tracing::debug!("{:?}", auth_request);

// Don't format secrets
format!("{}", secret_string);  // OK - shows ****
format!("{}", secret_string.expose_secret());  // NOT OK in logs

// Don't include secrets in errors
return Err(format!("Auth failed with password: {}", password));

// Don't use debug traits on sensitive structures
println!("{:#?}", login_request);
```

### ✅ DO
```rust
// Do use safe versions
tracing::debug!("{:?}", auth_request.to_safe());

// Do wrap sensitive fields
pub password: SecretString,

// Do mask before logging
tracing::error!(sql = redact_query(sql));

// Do use expose_secret only for API calls
let auth_header = format!("Bearer {}", token.expose_secret());
```

## Testing Your Code

### Check for credential leaks:
```bash
# Enable debug logging and check output
RUST_LOG=debug cargo test test_login 2>&1 | grep -E "(password|token)" | grep -v "****"
# Should return empty (no leaks)
```

### Run secrets module tests:
```bash
cargo test --lib secrets
```

### Run demo:
```bash
cargo run --example secrets_masking_demo
```

## Checklist for New Code

- [ ] All password/token/key fields use `SecretString` or are masked
- [ ] All logging of auth structures uses `.to_safe()`
- [ ] All SQL query logging uses `redact_query()`
- [ ] No `format!("{:?}", sensitive_struct)` without masking
- [ ] No credential values in error messages
- [ ] Tests verify no secrets in logs
- [ ] Code passes `cargo clippy`

## Emergency Response

If credentials are accidentally logged:

1. **Immediate**: Rotate all affected credentials
2. **Short-term**: Purge logs containing leaked credentials
3. **Investigation**: Identify how the leak occurred
4. **Prevention**: Add tests to prevent similar leaks
5. **Review**: Audit all logging code for similar issues

## Getting Help

- **Documentation**: See `SECRETS_MASKING.md` for detailed guide
- **Examples**: See `examples/secrets_masking_demo.rs`
- **Tests**: See tests in `src/secrets.rs`
- **Issues**: File bug at repository issue tracker
- **Security**: Contact security team for incidents

---

**Remember**: When in doubt, mask it out! It's better to over-redact than to leak credentials.
