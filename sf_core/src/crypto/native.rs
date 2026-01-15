//! Native cryptographic implementations using OpenSSL.
//!
//! This module provides FIPS-compliant crypto operations using OpenSSL/aws-lc.

use super::{
    AesCipher, CryptoError, DecryptionSnafu, EncryptionSnafu, InvalidPrivateKeySnafu, JwtSigner,
    JwtSigningSnafu, KeyCreationSnafu, PublicKeyExtractionSnafu, RandomGenerationSnafu,
    SecureRandom, Sha256Hasher, SystemTimeSnafu, UnsupportedKeySizeSnafu,
};
use crate::auth::extract_account_locator;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jwt::{Header, PKeyWithDigest, SignWithKey, Token};
use openssl::{
    hash::MessageDigest,
    pkey::PKey,
    rand::rand_bytes,
    rsa::Rsa,
    symm::{Cipher, decrypt, encrypt},
};
use serde::Serialize;
use snafu::OptionExt;
use std::time::{SystemTime, UNIX_EPOCH};

// AES constants
const AES_128_KEY_SIZE: usize = 16;
const AES_256_KEY_SIZE: usize = 32;

/// Native JWT signer using OpenSSL RSA.
#[derive(Default, Clone)]
pub struct NativeJwtSigner;

#[derive(Debug, Serialize)]
struct JwtClaim {
    sub: String,
    iss: String,
    iat: i64,
    exp: i64,
}

impl JwtSigner for NativeJwtSigner {
    fn sign_rs256(
        &self,
        private_key_pem: &[u8],
        passphrase: Option<&[u8]>,
        account: &str,
        username: &str,
    ) -> Result<String, CryptoError> {
        // Parse RSA private key
        let rsa = if let Some(pass) = passphrase {
            Rsa::private_key_from_pem_passphrase(private_key_pem, pass)
        } else {
            Rsa::private_key_from_pem(private_key_pem)
        }
        .ok()
        .context(InvalidPrivateKeySnafu)?;

        let private_key = PKey::from_rsa(rsa).ok().context(KeyCreationSnafu)?;

        // Extract public key and hash it
        let public_key_der = private_key
            .public_key_to_der()
            .ok()
            .context(PublicKeyExtractionSnafu)?;

        let mut hasher = openssl::sha::Sha256::new();
        hasher.update(&public_key_der);
        let public_key_hash = hasher.finish();
        let public_key_b64 = BASE64.encode(public_key_hash);

        let pkey_with_digest = PKeyWithDigest {
            digest: MessageDigest::sha256(),
            key: private_key,
        };

        // Create JWT header
        let header = Header {
            algorithm: jwt::AlgorithmType::Rs256,
            ..Default::default()
        };

        // Create claims
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .context(SystemTimeSnafu)?
            .as_secs() as i64;

        // Extract just the account locator (first segment before any dots)
        // Per Snowflake docs: JWT iss/sub must use account locator without region info
        let account_locator = extract_account_locator(account);
        let sub = format!("{}.{}", account_locator, username.to_uppercase());
        let iss = format!("{sub}.SHA256:{public_key_b64}");
        let claim = JwtClaim {
            sub,
            iss,
            iat: now,
            exp: now + 120,
        };

        // Create and sign token
        let token = Token::new(header, claim)
            .sign_with_key(&pkey_with_digest)
            .ok()
            .context(JwtSigningSnafu)?;

        Ok(token.as_str().to_string())
    }
}

/// Native AES cipher using OpenSSL.
#[derive(Default, Clone)]
pub struct NativeAesCipher;

impl NativeAesCipher {
    fn get_cbc_cipher(key_len: usize) -> Result<Cipher, CryptoError> {
        match key_len {
            AES_128_KEY_SIZE => Ok(Cipher::aes_128_cbc()),
            AES_256_KEY_SIZE => Ok(Cipher::aes_256_cbc()),
            _ => UnsupportedKeySizeSnafu { key_size: key_len }.fail(),
        }
    }

    fn get_ecb_cipher(key_len: usize) -> Result<Cipher, CryptoError> {
        match key_len {
            AES_128_KEY_SIZE => Ok(Cipher::aes_128_ecb()),
            AES_256_KEY_SIZE => Ok(Cipher::aes_256_ecb()),
            _ => UnsupportedKeySizeSnafu { key_size: key_len }.fail(),
        }
    }
}

impl AesCipher for NativeAesCipher {
    fn encrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Self::get_cbc_cipher(key.len())?;
        encrypt(cipher, key, Some(iv), data)
            .ok()
            .context(EncryptionSnafu {
                operation: "AES-CBC encryption".to_string(),
            })
    }

    fn decrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Self::get_cbc_cipher(key.len())?;
        decrypt(cipher, key, Some(iv), data)
            .ok()
            .context(DecryptionSnafu {
                operation: "AES-CBC decryption".to_string(),
            })
    }

    fn encrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Self::get_ecb_cipher(key.len())?;
        encrypt(cipher, key, None, data)
            .ok()
            .context(EncryptionSnafu {
                operation: "AES-ECB encryption".to_string(),
            })
    }

    fn decrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Self::get_ecb_cipher(key.len())?;
        decrypt(cipher, key, None, data)
            .ok()
            .context(DecryptionSnafu {
                operation: "AES-ECB decryption".to_string(),
            })
    }
}

/// Native SHA-256 hasher using OpenSSL.
#[derive(Default, Clone)]
pub struct NativeSha256Hasher;

impl Sha256Hasher for NativeSha256Hasher {
    fn hash(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = openssl::sha::Sha256::new();
        hasher.update(data);
        hasher.finish()
    }
}

/// Native secure random using OpenSSL.
#[derive(Default, Clone)]
pub struct NativeSecureRandom;

impl SecureRandom for NativeSecureRandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), CryptoError> {
        rand_bytes(dest).ok().context(RandomGenerationSnafu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash() {
        let hasher = NativeSha256Hasher;
        let hash = hasher.hash(b"hello world");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_secure_random() {
        let random = NativeSecureRandom;
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        random.fill_bytes(&mut buf1).unwrap();
        random.fill_bytes(&mut buf2).unwrap();
        // Random buffers should be different (with overwhelming probability)
        assert_ne!(buf1, buf2);
    }

    #[test]
    fn test_aes_cbc_roundtrip() {
        let cipher = NativeAesCipher;
        let random = NativeSecureRandom;

        let mut key = [0u8; 32];
        let mut iv = [0u8; 16];
        random.fill_bytes(&mut key).unwrap();
        random.fill_bytes(&mut iv).unwrap();

        let plaintext = b"Hello, World! This is a test message.";
        let encrypted = cipher.encrypt_cbc(&key, &iv, plaintext).unwrap();
        let decrypted = cipher.decrypt_cbc(&key, &iv, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_ecb_roundtrip() {
        let cipher = NativeAesCipher;
        let random = NativeSecureRandom;

        let mut key = [0u8; 32];
        random.fill_bytes(&mut key).unwrap();

        // ECB mode requires data to be block-aligned (or rely on padding)
        let plaintext = b"16 byte aligned!";
        let encrypted = cipher.encrypt_ecb(&key, plaintext).unwrap();
        let decrypted = cipher.decrypt_ecb(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
