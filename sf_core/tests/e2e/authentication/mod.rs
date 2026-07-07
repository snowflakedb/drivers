mod native_okta;
#[cfg(feature = "auth_oauth_e2e")]
mod oauth;
mod pat;
mod private_key_auth;
#[cfg(feature = "auth_spcs_e2e")]
mod spcs_token;
mod user_password;
#[cfg(feature = "auth_mfa_e2e")]
mod user_password_mfa;
mod workload_identity;
