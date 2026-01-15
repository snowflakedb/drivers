//! Cryptographic abstractions for platform-independent crypto operations.
//!
//! This module provides traits for cryptographic operations that can be implemented
//! by different backends (OpenSSL for native FIPS builds, RustCrypto for WASM).

use snafu::{Location, Snafu};

#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export the appropriate implementation based on features
#[cfg(feature = "native")]
pub use native::{
    NativeAesCipher as DefaultAesCipher, NativeJwtSigner as DefaultJwtSigner,
    NativeSecureRandom as DefaultSecureRandom, NativeSha256Hasher as DefaultSha256Hasher,
};

#[cfg(all(feature = "wasm", not(feature = "native")))]
pub use wasm::{
    WasmAesCipher as DefaultAesCipher, WasmJwtSigner as DefaultJwtSigner,
    WasmSecureRandom as DefaultSecureRandom, WasmSha256Hasher as DefaultSha256Hasher,
};

/// Trait for JWT signing operations.
///
/// Implementations must support RS256 (RSA-SHA256) signing as required by Snowflake.
pub trait JwtSigner: Send + Sync {
    /// Signs claims using RS256 algorithm with the provided private key.
    ///
    /// # Arguments
    /// * `private_key_pem` - PEM-encoded RSA private key
    /// * `passphrase` - Optional passphrase for encrypted private keys
    /// * `account` - Snowflake account identifier
    /// * `username` - Snowflake username
    ///
    /// # Returns
    /// The signed JWT token string
    fn sign_rs256(
        &self,
        private_key_pem: &[u8],
        passphrase: Option<&[u8]>,
        account: &str,
        username: &str,
    ) -> Result<String, CryptoError>;
}

/// Trait for AES encryption/decryption operations.
///
/// Supports both CBC and ECB modes as required by Snowflake file encryption.
pub trait AesCipher: Send + Sync {
    /// Encrypts data using AES-CBC mode with PKCS#7 padding.
    fn encrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>;

    /// Decrypts data using AES-CBC mode with PKCS#7 padding.
    fn decrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>;

    /// Encrypts data using AES-ECB mode with PKCS#7 padding.
    /// Note: ECB mode is used only for encrypting the file key with the master key.
    fn encrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>;

    /// Decrypts data using AES-ECB mode with PKCS#7 padding.
    fn decrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

/// Trait for SHA-256 hashing operations.
pub trait Sha256Hasher: Send + Sync {
    /// Computes SHA-256 hash of the input data.
    fn hash(&self, data: &[u8]) -> [u8; 32];
}

/// Trait for cryptographically secure random number generation.
pub trait SecureRandom: Send + Sync {
    /// Fills the destination buffer with cryptographically secure random bytes.
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), CryptoError>;
}

/// Errors that can occur during cryptographic operations.
#[derive(Snafu, Debug)]
pub enum CryptoError {
    #[snafu(display("Invalid private key format"))]
    InvalidPrivateKey {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to create key from RSA"))]
    KeyCreation {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to extract public key"))]
    PublicKeyExtraction {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to sign JWT token"))]
    JwtSigning {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Encryption operation failed: {operation}"))]
    Encryption {
        operation: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Decryption operation failed: {operation}"))]
    Decryption {
        operation: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported key size: {key_size} bytes"))]
    UnsupportedKeySize {
        key_size: usize,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Random number generation failed"))]
    RandomGeneration {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to get system time"))]
    SystemTime {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Helper to get the default crypto provider for the current platform.
pub fn default_crypto() -> CryptoProvider {
    CryptoProvider::default()
}

/// A bundle of all crypto implementations for convenience.
#[derive(Default)]
pub struct CryptoProvider {
    pub jwt_signer: DefaultJwtSigner,
    pub aes_cipher: DefaultAesCipher,
    pub sha256_hasher: DefaultSha256Hasher,
    pub secure_random: DefaultSecureRandom,
}
