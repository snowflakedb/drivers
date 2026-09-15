//! Cryptographic operations outside the TLS stack.
//!
//! The TLS provider and its CRL signature verification live in
//! [`crate::tls::crypto_module`]. This module is for everything else the driver
//! does with cryptography -- private-key loading today, and the remaining JWT,
//! DPoP and stage-encryption call sites as they consolidate here.
//!
//! The target state is that every security-relevant operation in this module
//! runs against the same AWS-LC build that carries TLS, so one compiled choice
//! answers "which module performed this operation" for the whole driver.

pub(crate) mod private_key;
