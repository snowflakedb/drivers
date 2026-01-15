//! WASM cryptographic implementations using RustCrypto.
//!
//! This module provides pure-Rust crypto operations for WASM builds.
#![allow(dead_code)]

use super::{
    AesCipher, CryptoError, DecryptionSnafu, EncryptionSnafu, InvalidPrivateKeySnafu, JwtSigner,
    PublicKeyExtractionSnafu, RandomGenerationSnafu, SecureRandom, Sha256Hasher, SystemTimeSnafu,
    UnsupportedKeySizeSnafu,
};
use crate::auth::extract_account_locator;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use snafu::OptionExt;

// RustCrypto imports
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit, block_padding::Pkcs7};
use sha2::{Digest, Sha256};

// AES constants
const AES_128_KEY_SIZE: usize = 16;
const AES_256_KEY_SIZE: usize = 32;
const AES_BLOCK_SIZE: usize = 16;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

// ECB mode types
type Aes128EcbEnc = ecb::Encryptor<aes::Aes128>;
type Aes128EcbDec = ecb::Decryptor<aes::Aes128>;
type Aes256EcbEnc = ecb::Encryptor<aes::Aes256>;
type Aes256EcbDec = ecb::Decryptor<aes::Aes256>;

/// WASM JWT signer using RustCrypto RSA.
#[derive(Default, Clone)]
pub struct WasmJwtSigner;

impl JwtSigner for WasmJwtSigner {
    fn sign_rs256(
        &self,
        private_key_pem: &[u8],
        passphrase: Option<&[u8]>,
        account: &str,
        username: &str,
    ) -> Result<String, CryptoError> {
        use pkcs8::DecodePrivateKey;
        use pkcs8::EncryptedPrivateKeyInfo;
        use pkcs8::SecretDocument;
        use pkcs8::der::Decode;
        use rsa::RsaPrivateKey;
        use rsa::pkcs1v15::SigningKey;
        use rsa::signature::{SignatureEncoding, Signer};

        // Parse the private key
        let private_key_str = std::str::from_utf8(private_key_pem)
            .ok()
            .context(InvalidPrivateKeySnafu)?;

        let private_key = if let Some(pass) = passphrase {
            // For encrypted PKCS#8 keys, we need to:
            // 1. Decode PEM to get raw DER bytes
            // 2. Parse as EncryptedPrivateKeyInfo
            // 3. Decrypt with passphrase
            // 4. Parse decrypted DER as RSA key

            // Manually decode PEM
            let pem_lines: Vec<&str> = private_key_str.lines().collect();
            let der_b64: String = pem_lines
                .iter()
                .filter(|l| !l.starts_with("-----"))
                .map(|s| s.trim())
                .collect();

            let der_bytes = BASE64
                .decode(&der_b64)
                .ok()
                .context(InvalidPrivateKeySnafu)?;

            // Parse as encrypted key info
            let encrypted = EncryptedPrivateKeyInfo::from_der(&der_bytes)
                .ok()
                .context(InvalidPrivateKeySnafu)?;

            // Decrypt
            let decrypted: SecretDocument = encrypted
                .decrypt(pass)
                .ok()
                .context(InvalidPrivateKeySnafu)?;

            // Parse as RSA key
            RsaPrivateKey::from_pkcs8_der(decrypted.as_bytes())
                .ok()
                .context(InvalidPrivateKeySnafu)?
        } else {
            RsaPrivateKey::from_pkcs8_pem(private_key_str)
                .ok()
                .context(InvalidPrivateKeySnafu)?
        };

        // Get public key DER for fingerprint
        use rsa::pkcs8::EncodePublicKey;
        let public_key = private_key.to_public_key();
        let public_key_der = public_key
            .to_public_key_der()
            .ok()
            .context(PublicKeyExtractionSnafu)?;

        // Hash the public key
        let hasher = WasmSha256Hasher;
        let public_key_hash = hasher.hash(public_key_der.as_bytes());
        let public_key_b64 = BASE64.encode(public_key_hash);

        // Create claims
        use std::time::{SystemTime, UNIX_EPOCH};
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

        // Build JWT manually (header.payload.signature)
        let header = r#"{"alg":"RS256","typ":"JWT"}"#;
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);

        let payload = format!(
            r#"{{"sub":"{sub}","iss":"{iss}","iat":{now},"exp":{}}}"#,
            now + 120
        );
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload);

        let signing_input = format!("{header_b64}.{payload_b64}");

        // Sign with RS256
        let signing_key: SigningKey<Sha256> = SigningKey::new(private_key);
        let signature = signing_key.sign(signing_input.as_bytes());
        let signature_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{signing_input}.{signature_b64}"))
    }
}

/// WASM AES cipher using RustCrypto.
#[derive(Default, Clone)]
pub struct WasmAesCipher;

impl AesCipher for WasmAesCipher {
    fn encrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match key.len() {
            AES_128_KEY_SIZE => {
                let cipher =
                    Aes128CbcEnc::new_from_slices(key, iv)
                        .ok()
                        .context(EncryptionSnafu {
                            operation: "AES-128-CBC init".to_string(),
                        })?;
                Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
            }
            AES_256_KEY_SIZE => {
                let cipher =
                    Aes256CbcEnc::new_from_slices(key, iv)
                        .ok()
                        .context(EncryptionSnafu {
                            operation: "AES-256-CBC init".to_string(),
                        })?;
                Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
            }
            _ => UnsupportedKeySizeSnafu {
                key_size: key.len(),
            }
            .fail(),
        }
    }

    fn decrypt_cbc(&self, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match key.len() {
            AES_128_KEY_SIZE => {
                let cipher =
                    Aes128CbcDec::new_from_slices(key, iv)
                        .ok()
                        .context(DecryptionSnafu {
                            operation: "AES-128-CBC init".to_string(),
                        })?;
                cipher
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-128-CBC decrypt".to_string(),
                    })
            }
            AES_256_KEY_SIZE => {
                let cipher =
                    Aes256CbcDec::new_from_slices(key, iv)
                        .ok()
                        .context(DecryptionSnafu {
                            operation: "AES-256-CBC init".to_string(),
                        })?;
                cipher
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-256-CBC decrypt".to_string(),
                    })
            }
            _ => UnsupportedKeySizeSnafu {
                key_size: key.len(),
            }
            .fail(),
        }
    }

    fn encrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match key.len() {
            AES_128_KEY_SIZE => {
                let cipher = Aes128EcbEnc::new_from_slice(key)
                    .ok()
                    .context(EncryptionSnafu {
                        operation: "AES-128-ECB init".to_string(),
                    })?;
                Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
            }
            AES_256_KEY_SIZE => {
                let cipher = Aes256EcbEnc::new_from_slice(key)
                    .ok()
                    .context(EncryptionSnafu {
                        operation: "AES-256-ECB init".to_string(),
                    })?;
                Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
            }
            _ => UnsupportedKeySizeSnafu {
                key_size: key.len(),
            }
            .fail(),
        }
    }

    fn decrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match key.len() {
            AES_128_KEY_SIZE => {
                let cipher = Aes128EcbDec::new_from_slice(key)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-128-ECB init".to_string(),
                    })?;
                cipher
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-128-ECB decrypt".to_string(),
                    })
            }
            AES_256_KEY_SIZE => {
                let cipher = Aes256EcbDec::new_from_slice(key)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-256-ECB init".to_string(),
                    })?;
                cipher
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .ok()
                    .context(DecryptionSnafu {
                        operation: "AES-256-ECB decrypt".to_string(),
                    })
            }
            _ => UnsupportedKeySizeSnafu {
                key_size: key.len(),
            }
            .fail(),
        }
    }
}

/// WASM SHA-256 hasher using RustCrypto.
#[derive(Default, Clone)]
pub struct WasmSha256Hasher;

impl Sha256Hasher for WasmSha256Hasher {
    fn hash(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

/// WASM secure random using getrandom.
#[derive(Default, Clone)]
pub struct WasmSecureRandom;

impl SecureRandom for WasmSecureRandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), CryptoError> {
        getrandom::getrandom(dest)
            .ok()
            .context(RandomGenerationSnafu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_key_jwt_fingerprint() {
        // Test that the fingerprint matches what openssl produces
        use pkcs8::DecodePrivateKey;
        use pkcs8::EncryptedPrivateKeyInfo;
        use pkcs8::SecretDocument;
        use pkcs8::der::Decode;
        use rsa::RsaPrivateKey;
        use rsa::pkcs8::EncodePublicKey;

        let encrypted_key = include_str!("../../tests/test_data/test_encrypted_key.pem");

        // Decrypt and get public key
        let pem_lines: Vec<&str> = encrypted_key.lines().collect();
        let der_b64: String = pem_lines
            .iter()
            .filter(|l| !l.starts_with("-----"))
            .map(|s| s.trim())
            .collect();

        let der_bytes = BASE64.decode(&der_b64).expect("base64 decode");
        let encrypted = EncryptedPrivateKeyInfo::from_der(&der_bytes).expect("parse encrypted");
        let decrypted: SecretDocument = encrypted.decrypt(b"ai loves universal").expect("decrypt");
        let private_key = RsaPrivateKey::from_pkcs8_der(decrypted.as_bytes()).expect("parse key");
        let public_key = private_key.to_public_key();
        let public_key_der = public_key.to_public_key_der().expect("encode public key");

        // Hash and base64 encode
        let hasher = WasmSha256Hasher;
        let hash = hasher.hash(public_key_der.as_bytes());
        let fingerprint = BASE64.encode(hash);

        // The expected fingerprint from openssl
        // Command: openssl rsa -in key.pem -pubout | openssl rsa -pubin -outform DER | openssl dgst -sha256 -binary | base64
        // Result: y/X4TRUiIxiOuBJUMLB/hI9qiwNZbgxQEdjj4TDe2iI=
        let expected = "y/X4TRUiIxiOuBJUMLB/hI9qiwNZbgxQEdjj4TDe2iI=";

        eprintln!("Generated fingerprint: {}", fingerprint);
        eprintln!("Expected fingerprint:  {}", expected);
        eprintln!("DER length: {}", public_key_der.as_bytes().len());

        assert_eq!(fingerprint, expected, "Fingerprint should match OpenSSL");
    }

    #[test]
    fn test_sha256_hash() {
        let hasher = WasmSha256Hasher;
        let hash = hasher.hash(b"hello world");
        assert_eq!(hash.len(), 32);
        // Known SHA256 of "hello world"
        let expected = [
            0xb9, 0x4d, 0x27, 0xb9, 0x93, 0x4d, 0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d,
            0xab, 0xfa, 0xc4, 0x84, 0xef, 0xe3, 0x7a, 0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac,
            0xe2, 0xef, 0xcd, 0xe9,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_secure_random() {
        let random = WasmSecureRandom;
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        random.fill_bytes(&mut buf1).unwrap();
        random.fill_bytes(&mut buf2).unwrap();
        assert_ne!(buf1, buf2);
    }

    #[test]
    fn test_aes_cbc_roundtrip() {
        let cipher = WasmAesCipher;
        let random = WasmSecureRandom;

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
        let cipher = WasmAesCipher;
        let random = WasmSecureRandom;

        let mut key = [0u8; 32];
        random.fill_bytes(&mut key).unwrap();

        let plaintext = b"16 byte aligned!";
        let encrypted = cipher.encrypt_ecb(&key, plaintext).unwrap();
        let decrypted = cipher.decrypt_ecb(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
