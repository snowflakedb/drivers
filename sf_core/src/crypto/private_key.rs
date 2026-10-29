//! Key-pair authentication private-key loading: PEM/DER in, an AWS-LC signing
//! key out.
//!
//! Replaces OpenSSL's `Rsa::private_key_from_pem[_passphrase]`. Handles the
//! shapes the drivers accept -- unencrypted PKCS#8, unencrypted PKCS#1 (RSA),
//! unencrypted SEC1 (EC), and PBES2-encrypted PKCS#8 -- and produces a
//! [`SigningKey`]: an RSA key, or an ECDSA key on P-256, P-384 or P-521.
//!
//! # Where the cryptography happens
//!
//! AWS-LC has no encrypted-private-key API at all, so the PBES2 envelope has to
//! be opened here. The split is deliberate:
//!
//! - **Parsing** (ASN.1: KDF identity, salt, iteration count, PRF, cipher, IV)
//!   is done by `pkcs5`, used parse-only with its cipher features switched off.
//!   Reading a structure is not a cryptographic operation, so nothing about the
//!   compliance boundary turns on who does it.
//! - **Key derivation** (PBKDF2) and **AES-CBC decryption** run in AWS-LC.
//!   These are approved operations and they belong inside the module.
//! - **3DES-CBC decryption**, on a standard build, runs in the `des` crate.
//!   3DES is not an approved algorithm, so a `fips` build rejects a PBES2
//!   key whose cipher is 3DES instead of decrypting it.
//!
//! # Where the builds differ
//!
//! Two customer-supplied formats load on a standard build and are refused on
//! a `fips` build. AES-128/192/256-CBC PKCS#8 loads on both.
//!
//! - PBES2 with 3DES-CBC, which `openssl pkcs8 -topk8 -v2 des3` writes. The
//!   KDF is PBKDF2 inside AWS-LC. A standard build decrypts the ciphertext
//!   with the `des` crate. A `fips` build returns
//!   [`PrivateKeyError::TripleDesEncrypted`].
//! - Traditional encrypted PEM (`-----BEGIN RSA PRIVATE KEY-----` or
//!   `-----BEGIN EC PRIVATE KEY-----` with `Proc-Type: 4,ENCRYPTED`), which
//!   `openssl rsa -aes256`, `openssl genrsa -aes256` and `openssl ec -aes256`
//!   emit. The format derives its key with OpenSSL's
//!   MD5 `EVP_BytesToKey`, and AWS-LC exposes no MD5. See [`legacy_pem`].
//!
//! Both refusal errors name the build and the conversion command
//! `openssl pkcs8 -topk8 -v2 aes-256-cbc`. A standard build makes no FIPS
//! claim, so it keeps both formats. Recorded as F16 in the compliance plan.

use aws_lc_rs::cipher::{
    AES_128, AES_192, AES_256, DecryptionContext, PaddedBlockDecryptingKey, UnboundCipherKey,
};
use aws_lc_rs::encoding::{AsDer, PublicKeyX509Der};
use aws_lc_rs::iv::{FixedLength, IV_LEN_128_BIT};
use aws_lc_rs::pbkdf2;
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{
    ECDSA_P256_SHA256_FIXED_SIGNING, ECDSA_P384_SHA384_FIXED_SIGNING,
    ECDSA_P521_SHA512_FIXED_SIGNING, EcdsaKeyPair, EcdsaSigningAlgorithm, KeyPair,
    RSA_PKCS1_SHA256, RsaKeyPair,
};
use const_oid::ObjectIdentifier;
use pkcs8::pkcs5::{self, pbes2};
use snafu::{Location, OptionExt, ResultExt, Snafu};

use crate::sensitive::Sensitive;

/// PEM labels we accept, in the forms the drivers have historically seen.
const LABEL_PKCS8: &str = "PRIVATE KEY";
const LABEL_PKCS8_ENCRYPTED: &str = "ENCRYPTED PRIVATE KEY";
const LABEL_PKCS1: &str = "RSA PRIVATE KEY";
const LABEL_SEC1: &str = "EC PRIVATE KEY";

const OID_RSA_ENCRYPTION: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
const OID_EC_PUBLIC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
const OID_PRIME256V1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
const OID_SECP384R1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.132.0.34");
const OID_SECP521R1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.132.0.35");

/// The ECDSA curves Snowflake accepts for key-pair authentication, each tied
/// to the one JWS algorithm (RFC 7518 §3.4) that pairs it with its hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EcCurve {
    P256,
    P384,
    P521,
}

impl EcCurve {
    fn from_oid(oid: ObjectIdentifier) -> Option<Self> {
        match oid {
            OID_PRIME256V1 => Some(EcCurve::P256),
            OID_SECP384R1 => Some(EcCurve::P384),
            OID_SECP521R1 => Some(EcCurve::P521),
            _ => None,
        }
    }

    /// The `FIXED` variants emit the raw `r || s` signature a JWS requires,
    /// not the ASN.1 `ECDSA-Sig-Value` that `*_ASN1_SIGNING` produces.
    fn signing_algorithm(self) -> &'static EcdsaSigningAlgorithm {
        match self {
            EcCurve::P256 => &ECDSA_P256_SHA256_FIXED_SIGNING,
            EcCurve::P384 => &ECDSA_P384_SHA384_FIXED_SIGNING,
            EcCurve::P521 => &ECDSA_P521_SHA512_FIXED_SIGNING,
        }
    }

    fn jwt_algorithm(self) -> &'static str {
        match self {
            EcCurve::P256 => "ES256",
            EcCurve::P384 => "ES384",
            EcCurve::P521 => "ES512",
        }
    }
}

/// A private key usable for a key-pair authentication JWT.
#[derive(Debug)]
pub(crate) enum SigningKey {
    Rsa(RsaKeyPair),
    Ecdsa { key: EcdsaKeyPair, curve: EcCurve },
}

impl SigningKey {
    /// The JWS `alg` header value this key signs with.
    pub(crate) fn jwt_algorithm(&self) -> &'static str {
        match self {
            SigningKey::Rsa(_) => "RS256",
            SigningKey::Ecdsa { curve, .. } => curve.jwt_algorithm(),
        }
    }

    /// The public half as X.509 `SubjectPublicKeyInfo` DER, which is what
    /// Snowflake fingerprints for both RSA and EC keys.
    pub(crate) fn public_key_spki_der(&self) -> Result<Vec<u8>, aws_lc_rs::error::Unspecified> {
        let der: PublicKeyX509Der<'static> = match self {
            SigningKey::Rsa(key) => key.public_key().as_der()?,
            SigningKey::Ecdsa { key, .. } => key.public_key().as_der()?,
        };
        Ok(der.as_ref().to_vec())
    }

    /// Sign `message` with the algorithm named by [`Self::jwt_algorithm`].
    pub(crate) fn sign(&self, message: &[u8]) -> Result<Vec<u8>, aws_lc_rs::error::Unspecified> {
        let rng = SystemRandom::new();
        match self {
            SigningKey::Rsa(key) => {
                let mut signature = vec![0u8; key.public_modulus_len()];
                key.sign(&RSA_PKCS1_SHA256, &rng, message, &mut signature)?;
                Ok(signature)
            }
            SigningKey::Ecdsa { key, .. } => Ok(key.sign(&rng, message)?.as_ref().to_vec()),
        }
    }
}

/// Load an RSA or ECDSA signing key from PEM or DER.
///
/// `passphrase` is required for an encrypted key and ignored for an
/// unencrypted one, matching OpenSSL's behaviour.
pub(crate) fn load_signing_key(
    key: &[u8],
    passphrase: Option<&str>,
) -> Result<SigningKey, PrivateKeyError> {
    match decode_pem(key)? {
        Some(Pem::Plain { label, der }) => from_labelled_der(&label, &der, passphrase),
        Some(Pem::Legacy {
            label,
            dek_info,
            body,
        }) => from_legacy_pem(&label, &dek_info, &body, passphrase),
        // No PEM armour: the input is DER. This is a real input path, not just
        // a fallback -- the Python connector supplies raw DER bytes -- and that
        // DER may itself be an encrypted PKCS#8 envelope, so it gets the same
        // treatment as the armoured form.
        None => from_bare_der(key, passphrase),
    }
}

/// Traditional encrypted PEM: decrypt with the `DEK-Info` parameters, then
/// parse the plaintext as whatever the label says it is.
///
/// Split by build. See [`legacy_pem`]: the format's KDF is MD5, which a
/// `fips` build does not run. PBES2 3DES is the other format refused only
/// on that build.
#[cfg(not(feature = "fips"))]
fn from_legacy_pem(
    label: &str,
    dek_info: &str,
    body: &[u8],
    passphrase: Option<&str>,
) -> Result<SigningKey, PrivateKeyError> {
    let passphrase = passphrase.with_context(|| PassphraseRequiredSnafu)?;
    let der = legacy_pem::decrypt(dek_info, body, passphrase)?;
    // The passphrase is spent; the plaintext is an ordinary unencrypted body.
    from_labelled_der(label, der.reveal(), None)
}

#[cfg(feature = "fips")]
fn from_legacy_pem(
    _label: &str,
    _dek_info: &str,
    _body: &[u8],
    _passphrase: Option<&str>,
) -> Result<SigningKey, PrivateKeyError> {
    LegacyEncryptedPemSnafu.fail()
}

/// The PEM label matching `der`'s actual format, validating it along the way.
///
/// Lets the config layer armour raw DER without guessing: a PKCS#1 body
/// labelled `PRIVATE KEY` would be handed to the PKCS#8 parser and rejected,
/// and an encrypted envelope labelled the same way would never reach the
/// decryption path at all.
///
/// The parse is deliberately repeated later by [`load_signing_key`]. Validating
/// here is what keeps a malformed key an immediate, precise configuration
/// error instead of a confusing failure at first login.
pub(crate) fn der_pem_label(der: &[u8]) -> Result<&'static str, PrivateKeyError> {
    if pkcs8::EncryptedPrivateKeyInfo::try_from(der).is_ok() {
        return Ok(LABEL_PKCS8_ENCRYPTED);
    }
    parse_unlabelled_der(der).map(|(label, _)| label)
}

fn from_labelled_der(
    label: &str,
    der: &[u8],
    passphrase: Option<&str>,
) -> Result<SigningKey, PrivateKeyError> {
    match label {
        LABEL_PKCS8 => from_pkcs8(der, "PKCS#8"),
        LABEL_PKCS1 => RsaKeyPair::from_der(der)
            .map(SigningKey::Rsa)
            .context(KeyRejectedSnafu { format: "PKCS#1" }),
        LABEL_SEC1 => from_sec1(der).context(KeyRejectedSnafu { format: "SEC1" }),
        LABEL_PKCS8_ENCRYPTED => unwrap_encrypted(der, passphrase),
        other => UnsupportedPemLabelSnafu {
            label: other.to_string(),
        }
        .fail(),
    }
}

fn from_bare_der(der: &[u8], passphrase: Option<&str>) -> Result<SigningKey, PrivateKeyError> {
    // Encrypted first. `EncryptedPrivateKeyInfo` is a distinctive shape, so
    // testing for it up front means a missing passphrase is reported as
    // exactly that, rather than as a generic "key rejected" from the plain
    // parsers failing on an envelope they were never given a chance to open.
    if pkcs8::EncryptedPrivateKeyInfo::try_from(der).is_ok() {
        return unwrap_encrypted(der, passphrase);
    }
    parse_unlabelled_der(der).map(|(_, key)| key)
}

/// Parse unencrypted DER of unknown format, returning the PEM label it
/// matches alongside the key.
///
/// PKCS#8 `PrivateKeyInfo`, PKCS#1 `RSAPrivateKey` and SEC1 `ECPrivateKey`
/// differ in their second element (a SEQUENCE, an INTEGER and an OCTET STRING
/// respectively), so a well-formed body is accepted by exactly one parser.
fn parse_unlabelled_der(der: &[u8]) -> Result<(&'static str, SigningKey), PrivateKeyError> {
    if pkcs8::PrivateKeyInfo::try_from(der).is_ok() {
        return from_pkcs8(der, "PKCS#8").map(|key| (LABEL_PKCS8, key));
    }
    if let Ok(key) = RsaKeyPair::from_der(der) {
        return Ok((LABEL_PKCS1, SigningKey::Rsa(key)));
    }
    from_sec1(der)
        .map(|key| (LABEL_SEC1, key))
        .context(KeyRejectedSnafu {
            format: "DER (tried PKCS#8, PKCS#1 and SEC1)",
        })
}

/// Parse a PKCS#8 `PrivateKeyInfo`, choosing the key type from its
/// algorithm identifier and, for EC, the curve from its parameters.
fn from_pkcs8(der: &[u8], format: &'static str) -> Result<SigningKey, PrivateKeyError> {
    let algorithm = match pkcs8::PrivateKeyInfo::try_from(der) {
        Ok(info) => info.algorithm,
        Err(_) => {
            return RsaKeyPair::from_pkcs8(der)
                .map(SigningKey::Rsa)
                .context(KeyRejectedSnafu { format });
        }
    };
    if algorithm.oid == OID_RSA_ENCRYPTION {
        return RsaKeyPair::from_pkcs8(der)
            .map(SigningKey::Rsa)
            .context(KeyRejectedSnafu { format });
    }
    if algorithm.oid != OID_EC_PUBLIC_KEY {
        return UnsupportedKeyAlgorithmSnafu {
            algorithm: algorithm.oid.to_string(),
        }
        .fail();
    }
    let curve_oid = algorithm.parameters_oid().ok();
    let curve = curve_oid
        .and_then(EcCurve::from_oid)
        .with_context(|| UnsupportedEcCurveSnafu {
            curve: curve_oid
                .map_or_else(|| "(not a named curve)".to_string(), |oid| oid.to_string()),
        })?;
    EcdsaKeyPair::from_pkcs8(curve.signing_algorithm(), der)
        .map(|key| SigningKey::Ecdsa { key, curve })
        .context(KeyRejectedSnafu { format })
}

/// Parse a SEC1 (RFC 5915) `ECPrivateKey`. The curve is found by trying each
/// supported one: AWS-LC rejects a key whose curve differs from the
/// algorithm it is asked to load it for.
fn from_sec1(der: &[u8]) -> Result<SigningKey, aws_lc_rs::error::KeyRejected> {
    let load = |curve: EcCurve| {
        EcdsaKeyPair::from_private_key_der(curve.signing_algorithm(), der)
            .map(|key| SigningKey::Ecdsa { key, curve })
    };
    load(EcCurve::P256)
        .or_else(|_| load(EcCurve::P384))
        .or_else(|_| load(EcCurve::P521))
}

fn unwrap_encrypted(der: &[u8], passphrase: Option<&str>) -> Result<SigningKey, PrivateKeyError> {
    let passphrase = passphrase.with_context(|| PassphraseRequiredSnafu)?;
    let plaintext = decrypt_pkcs8(der, passphrase)?;
    from_pkcs8(plaintext.reveal(), "decrypted PKCS#8")
}

/// Decrypt a PBES2-encrypted `EncryptedPrivateKeyInfo` into PKCS#8 DER.
fn decrypt_pkcs8(der: &[u8], passphrase: &str) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
    let info = pkcs8::EncryptedPrivateKeyInfo::try_from(der).map_err(|e| {
        EnvelopeParseSnafu {
            detail: e.to_string(),
        }
        .build()
    })?;

    let params = match &info.encryption_algorithm {
        pkcs5::EncryptionScheme::Pbes2(params) => params,
        // PBES1 uses MD5/SHA-1-based KDFs with DES or RC2. OpenSSL accepted it;
        // it predates PBES2 by two decades and no Snowflake tooling emits it.
        // `EncryptionScheme` is `#[non_exhaustive]`, so anything new lands here
        // too -- refusing an unknown wrapper is the right default.
        _ => return UnsupportedPbes1Snafu.fail(),
    };

    let key = derive_key(&params.kdf, passphrase, params.encryption.key_size())?;
    decrypt_payload(&params.encryption, key.reveal(), info.encrypted_data)
}

/// PBKDF2 via AWS-LC. scrypt is rejected rather than farmed out.
fn derive_key(
    kdf: &pbes2::Kdf<'_>,
    passphrase: &str,
    key_len: usize,
) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
    let params = match kdf {
        pbes2::Kdf::Pbkdf2(p) => p,
        // OpenSSL 3 can emit scrypt with `-v2 ... -scrypt`. AWS-LC exposes
        // scrypt only under `unstable`, so rejecting it beats depending on an
        // unstable API for a format nothing in this ecosystem produces. `Kdf`
        // is `#[non_exhaustive]`, so unknown KDFs are refused here as well.
        _ => return UnsupportedScryptSnafu.fail(),
    };

    let prf = match params.prf {
        pbes2::Pbkdf2Prf::HmacWithSha1 => pbkdf2::PBKDF2_HMAC_SHA1,
        pbes2::Pbkdf2Prf::HmacWithSha256 => pbkdf2::PBKDF2_HMAC_SHA256,
        pbes2::Pbkdf2Prf::HmacWithSha384 => pbkdf2::PBKDF2_HMAC_SHA384,
        pbes2::Pbkdf2Prf::HmacWithSha512 => pbkdf2::PBKDF2_HMAC_SHA512,
        // SHA-224 exists in the ASN.1 but AWS-LC offers no PBKDF2-HMAC-SHA224.
        // `Pbkdf2Prf` is `#[non_exhaustive]`, so anything unrecognised is
        // refused here rather than silently mapped to a different hash.
        _ => {
            return UnsupportedPrfSnafu {
                prf: "SHA-224 or unrecognised",
            }
            .fail();
        }
    };

    // A zero iteration count would make `derive` panic rather than error.
    let iterations =
        std::num::NonZeroU32::new(params.iteration_count).with_context(|| ZeroIterationsSnafu)?;

    let mut out = vec![0u8; key_len];
    pbkdf2::derive(
        prf,
        iterations,
        params.salt,
        passphrase.as_bytes(),
        &mut out,
    );
    Ok(out.into())
}

fn decrypt_payload(
    scheme: &pbes2::EncryptionScheme<'_>,
    key: &[u8],
    ciphertext: &[u8],
) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
    match scheme {
        pbes2::EncryptionScheme::Aes128Cbc { iv } => aes_cbc_decrypt(&AES_128, key, iv, ciphertext),
        pbes2::EncryptionScheme::Aes192Cbc { iv } => aes_cbc_decrypt(&AES_192, key, iv, ciphertext),
        pbes2::EncryptionScheme::Aes256Cbc { iv } => aes_cbc_decrypt(&AES_256, key, iv, ciphertext),
        #[cfg(not(feature = "fips"))]
        pbes2::EncryptionScheme::DesEde3Cbc { iv } => des_ede3_cbc_decrypt(key, iv, ciphertext),
        #[cfg(feature = "fips")]
        pbes2::EncryptionScheme::DesEde3Cbc { .. } => TripleDesEncryptedSnafu.fail(),
        // Single-DES has no arm because its `pkcs5` variant is behind the
        // `des-insecure` feature we deliberately leave off: a 56-bit key is
        // brute-forceable, so such a key falls through to the catch-all and is
        // refused rather than carried forward.
        other => UnsupportedCipherSnafu {
            cipher: format!("{:?}", other.oid()),
        }
        .fail(),
    }
}

/// AES-CBC/PKCS#7 inside AWS-LC.
fn aes_cbc_decrypt(
    algorithm: &'static aws_lc_rs::cipher::Algorithm,
    key: &[u8],
    iv: &[u8; IV_LEN_128_BIT],
    ciphertext: &[u8],
) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
    let unbound = UnboundCipherKey::new(algorithm, key).context(CryptoSnafu {
        operation: "binding the derived AES key",
    })?;
    let decrypting = PaddedBlockDecryptingKey::cbc_pkcs7(unbound).context(CryptoSnafu {
        operation: "initializing AES-CBC decryption of the private key",
    })?;
    let mut buf = ciphertext.to_vec();
    // Take only the plaintext *length* from the borrow, then move `buf` itself
    // into `Sensitive`. `plaintext.to_vec()` would leave `buf` holding a second,
    // un-zeroized copy of the key until its allocation is reused: `decrypt`
    // works in place, so `buf` is the plaintext, and only the `Sensitive`
    // wrapper zeroizes on drop. The bytes past `len` are PKCS#7 padding, not key
    // material, so leaving them in spare capacity discloses nothing.
    let plaintext_len = decrypting
        .decrypt(&mut buf, DecryptionContext::Iv128(FixedLength::from(*iv)))
        .context(CryptoSnafu {
            operation: "decrypting the private key (wrong passphrase?)",
        })?
        .len();
    buf.truncate(plaintext_len);
    Ok(buf.into())
}

/// 3DES-CBC/PKCS#7. A `fips` build never reaches this: that cipher is
/// rejected in [`decrypt_payload`].
#[cfg(not(feature = "fips"))]
fn des_ede3_cbc_decrypt(
    key: &[u8],
    iv: &[u8; 8],
    ciphertext: &[u8],
) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
    use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};

    type Decryptor = cbc::Decryptor<des::TdesEde3>;

    let decryptor = Decryptor::new_from_slices(key, iv).map_err(|_| {
        CryptoBackendSnafu {
            operation: "initializing 3DES-CBC decryption of the private key",
        }
        .build()
    })?;
    let mut buf = ciphertext.to_vec();
    // Same reasoning as `aes_cbc_decrypt`: take the length from the borrow and
    // move `buf`, rather than copying the plaintext out and dropping `buf`
    // un-zeroized.
    let plaintext_len = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| {
            CryptoBackendSnafu {
                operation: "decrypting the private key with 3DES-CBC (wrong passphrase?)",
            }
            .build()
        })?
        .len();
    buf.truncate(plaintext_len);
    Ok(buf.into())
}

/// What a block of PEM armour turned out to contain.
enum Pem {
    /// Ordinary armour: a label and the DER it wraps.
    Plain { label: String, der: Vec<u8> },
    /// Traditional encrypted PEM, whose cipher and IV live in `DEK-Info`
    /// headers rather than in the DER. `body` is still ciphertext.
    Legacy {
        label: String,
        dek_info: String,
        body: Vec<u8>,
    },
}

/// Split PEM armour into its label and body.
///
/// `Ok(None)` means the input carried no armour and should be treated as DER.
/// Deliberately hand-rolled rather than via `rustls-pemfile`: that crate keys
/// off the label to decide what it parsed and has no variant for
/// `ENCRYPTED PRIVATE KEY`, so it cannot report the one case that matters here.
fn decode_pem(input: &[u8]) -> Result<Option<Pem>, PrivateKeyError> {
    let text = match std::str::from_utf8(input) {
        Ok(text) => text,
        // Invalid UTF-8 cannot be PEM; it is DER (or nothing useful).
        Err(_) => return Ok(None),
    };
    let text = skip_leading_ec_parameters(text);
    let Some(begin) = text.find("-----BEGIN ") else {
        return Ok(None);
    };
    let after_begin = &text[begin + "-----BEGIN ".len()..];
    let Some(label_end) = after_begin.find("-----") else {
        return MalformedPemSnafu {
            detail: "BEGIN line is not terminated by `-----`",
        }
        .fail();
    };
    let label = after_begin[..label_end].trim().to_string();

    let body_start = begin + "-----BEGIN ".len() + label_end + "-----".len();
    let rest = &text[body_start..];
    let Some(end) = rest.find("-----END ") else {
        return MalformedPemSnafu {
            detail: "no `-----END` line",
        }
        .fail();
    };

    // The END label has to match the BEGIN label. Without this, armour closed
    // by a different marker still "parses": the body is taken verbatim and
    // handed to a parser chosen by the BEGIN label, so a truncated file that
    // runs into the next object's footer fails later as "key rejected as
    // PKCS#8" rather than as the framing error it is.
    let after_end = &rest[end + "-----END ".len()..];
    let end_label = after_end
        .find("-----")
        .map(|i| after_end[..i].trim())
        .with_context(|| MalformedPemSnafu {
            detail: "END line is not terminated by `-----`",
        })?;
    if end_label != label {
        return MalformedPemSnafu {
            detail: format!("BEGIN label `{label}` does not match END label `{end_label}`"),
        }
        .fail();
    }

    let body_text = &rest[..end];

    // Traditional PKCS#1 encryption ("Proc-Type: 4,ENCRYPTED" plus "DEK-Info")
    // keeps its parameters in PEM headers rather than in ASN.1, so the body has
    // a header block before the base64 and is ciphertext once decoded. Detected
    // by exactly the condition that used to reject it outright, so ordinary
    // armour reaches the base64 path on an unchanged code path.
    if body_text.contains("Proc-Type:") && body_text.contains("ENCRYPTED") {
        let dek_info = body_text
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix("DEK-Info:"))
            .map(str::trim)
            .with_context(|| MalformedPemSnafu {
                detail: "`Proc-Type: 4,ENCRYPTED` without a `DEK-Info` header",
            })?
            .to_string();
        let b64_text = body_after_headers(body_text).with_context(|| MalformedPemSnafu {
            detail: "encrypted PEM headers are not followed by a blank line",
        })?;
        return Ok(Some(Pem::Legacy {
            label,
            dek_info,
            body: decode_b64(b64_text)?,
        }));
    }

    Ok(Some(Pem::Plain {
        label,
        der: decode_b64(body_text)?,
    }))
}

/// `openssl ecparam -genkey` writes an `EC PARAMETERS` block ahead of the
/// key. The key names its own curve, so that block carries nothing the
/// loader needs.
fn skip_leading_ec_parameters(text: &str) -> &str {
    const BEGIN: &str = "-----BEGIN EC PARAMETERS-----";
    const END: &str = "-----END EC PARAMETERS-----";
    if text.find("-----BEGIN ") != text.find(BEGIN) {
        return text;
    }
    match text.find(END) {
        Some(end) => &text[end + END.len()..],
        None => text,
    }
}

/// The base64 payload following a PEM header block.
///
/// Headers end at the first blank line *after* at least one header, which is
/// what distinguishes that blank line from the newline immediately after the
/// `-----BEGIN ...-----` marker -- the body starts with that newline, so a
/// naive "first empty line" search matches straight away and consumes nothing.
fn body_after_headers(body: &str) -> Option<&str> {
    let mut offset = 0usize;
    let mut seen_header = false;
    for line in body.split_inclusive('\n') {
        offset += line.len();
        if line.trim().is_empty() {
            if seen_header {
                return Some(&body[offset..]);
            }
        } else {
            seen_header = true;
        }
    }
    None
}

fn decode_b64(text: &str) -> Result<Vec<u8>, PrivateKeyError> {
    use base64::Engine as _;

    let b64: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| {
            MalformedPemSnafu {
                detail: format!("body is not valid base64: {e}"),
            }
            .build()
        })
}

/// Traditional encrypted PEM, supported only in non-FIPS builds.
///
/// Gated because of the KDF, not the cipher. The format derives its key with
/// OpenSSL's `EVP_BytesToKey`, which is MD5-based -- MD5 is definitional to
/// the format, so there is no variant of this that runs inside AWS-LC. A
/// `fips` build therefore cannot read these keys and says so, while a
/// standard build makes no FIPS claim and keeps the compatibility the OpenSSL
/// loader had.
///
/// PBES2 3DES is the other format a `fips` build refuses. Recorded as F16
/// in the compliance plan: a standard build makes no FIPS claim and keeps the
/// format.
///
/// The bulk decryption still runs in AWS-LC for AES and in `des` for 3DES.
/// Only the key derivation is MD5.
#[cfg(not(feature = "fips"))]
mod legacy_pem {
    use super::{
        AES_128, AES_192, AES_256, MalformedPemSnafu, PrivateKeyError, Sensitive,
        UnsupportedCipherSnafu, aes_cbc_decrypt, des_ede3_cbc_decrypt,
    };
    use snafu::OptionExt;

    /// Decrypt a traditional encrypted PEM body using its `DEK-Info` header.
    pub(super) fn decrypt(
        dek_info: &str,
        ciphertext: &[u8],
        passphrase: &str,
    ) -> Result<Sensitive<Vec<u8>>, PrivateKeyError> {
        let (algorithm, iv_hex) = dek_info
            .split_once(',')
            .with_context(|| MalformedPemSnafu {
                detail: format!("`DEK-Info: {dek_info}` has no `,<iv>`"),
            })?;
        let iv = hex::decode(iv_hex.trim()).map_err(|e| {
            MalformedPemSnafu {
                detail: format!("`DEK-Info` IV is not hex: {e}"),
            }
            .build()
        })?;

        // OpenSSL salts the KDF with the first 8 bytes of the IV and derives
        // only the key; the full IV comes from the header. See `PEM_do_header`,
        // which calls `EVP_BytesToKey(cipher, EVP_md5(), &iv[0], pass, len, 1,
        // key, NULL)` -- count 1, and a NULL iv output.
        let salt = iv.get(..8).with_context(|| MalformedPemSnafu {
            detail: "`DEK-Info` IV is shorter than the 8 bytes used as KDF salt",
        })?;

        match algorithm.trim().to_ascii_uppercase().as_str() {
            "AES-128-CBC" => {
                let key = evp_bytes_to_key(passphrase, salt, 16);
                aes_cbc_decrypt(&AES_128, key.reveal(), &iv_128(&iv)?, ciphertext)
            }
            "AES-192-CBC" => {
                let key = evp_bytes_to_key(passphrase, salt, 24);
                aes_cbc_decrypt(&AES_192, key.reveal(), &iv_128(&iv)?, ciphertext)
            }
            "AES-256-CBC" => {
                let key = evp_bytes_to_key(passphrase, salt, 32);
                aes_cbc_decrypt(&AES_256, key.reveal(), &iv_128(&iv)?, ciphertext)
            }
            "DES-EDE3-CBC" => {
                let key = evp_bytes_to_key(passphrase, salt, 24);
                let iv: [u8; 8] = iv.as_slice().try_into().map_err(|_| {
                    MalformedPemSnafu {
                        detail: format!("3DES needs an 8-byte IV, got {}", iv.len()),
                    }
                    .build()
                })?;
                des_ede3_cbc_decrypt(key.reveal(), &iv, ciphertext)
            }
            // Single-DES is refused for the same reason the PBES2 path leaves
            // `pkcs5`'s `des-insecure` feature off: a 56-bit key is
            // brute-forceable, so "OpenSSL read it" is not on its own a reason
            // to keep reading it. RC2 likewise.
            other => UnsupportedCipherSnafu {
                cipher: format!("{other} (traditional encrypted PEM)"),
            }
            .fail(),
        }
    }

    fn iv_128(iv: &[u8]) -> Result<[u8; 16], PrivateKeyError> {
        iv.try_into().map_err(|_| {
            MalformedPemSnafu {
                detail: format!("AES-CBC needs a 16-byte IV, got {}", iv.len()),
            }
            .build()
        })
    }

    /// OpenSSL's `EVP_BytesToKey` with MD5 and an iteration count of 1.
    ///
    /// `D_1 = MD5(pass || salt)`, then `D_i = MD5(D_{i-1} || pass || salt)`,
    /// concatenated until `key_len` bytes are available. Only the key is
    /// produced: the traditional PEM format takes its IV from the header
    /// instead of from the tail of this stream, which is why the caller passes
    /// the IV in separately.
    ///
    /// Hand-rolled because no crate in this graph exposes it and AWS-LC does
    /// not implement it. The construction is fixed by the on-disk format, so
    /// there is nothing to choose here and nothing to get creatively wrong --
    /// the round-trip tests decrypt keys produced by OpenSSL itself, which is
    /// the only check that matters.
    fn evp_bytes_to_key(passphrase: &str, salt: &[u8], key_len: usize) -> Sensitive<Vec<u8>> {
        use md5::{Digest as _, Md5};
        use zeroize::Zeroize as _;

        let mut key = Vec::with_capacity(key_len + Md5::output_size());
        let mut block: Vec<u8> = Vec::new();
        while key.len() < key_len {
            let mut hasher = Md5::new();
            hasher.update(&block);
            hasher.update(passphrase.as_bytes());
            hasher.update(salt);
            block.zeroize();
            block = hasher.finalize().to_vec();
            key.extend_from_slice(&block);
        }
        // `block` is derived key material in its own right, and unlike `key` it
        // never reaches a `Sensitive` wrapper.
        block.zeroize();
        key.truncate(key_len);
        key.into()
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum PrivateKeyError {
    #[snafu(display("Private key was rejected as {format}"))]
    KeyRejected {
        format: String,
        source: aws_lc_rs::error::KeyRejected,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Private key is encrypted but no passphrase was provided; set `private_key_password`"
    ))]
    PassphraseRequired {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported PEM label `{label}` for a private key"))]
    UnsupportedPemLabel {
        label: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Private key algorithm {algorithm} is not supported; use an RSA key or an \
         ECDSA key on P-256, P-384 or P-521"
    ))]
    UnsupportedKeyAlgorithm {
        algorithm: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "ECDSA private key curve {curve} is not supported; use P-256, P-384 or P-521"
    ))]
    UnsupportedEcCurve {
        curve: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Could not parse the encrypted private-key envelope: {detail}"))]
    EnvelopeParse {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Malformed PEM: {detail}"))]
    MalformedPem {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Only reachable in `fips` builds; standard builds read these keys.
    /// The message says which build the user is on, because otherwise "this
    /// key does not work" is indistinguishable from "this key is broken" for
    /// someone whose colleague on a default artifact is using it happily.
    #[snafu(display(
        "This private key uses the traditional encrypted-PEM format \
         (`Proc-Type: 4,ENCRYPTED`), whose key derivation is MD5-based. This is \
         a FIPS build, which cannot derive a key with MD5; standard builds of \
         this driver read these keys normally. Either use a standard build, or \
         convert the key with: \
         openssl pkcs8 -topk8 -v2 aes-256-cbc -in <key> -out <key>.p8"
    ))]
    LegacyEncryptedPem {
        #[snafu(implicit)]
        location: Location,
    },

    /// Only reachable in `fips` builds; standard builds unwrap these keys.
    /// The message names the build for the same reason as [`Self::LegacyEncryptedPem`].
    #[snafu(display(
        "This private key is encrypted with 3DES-CBC. This is a FIPS build, \
         which does not unwrap 3DES; standard builds of this driver read these \
         keys normally. Either use a standard build, or convert the key with: \
         openssl pkcs8 -topk8 -v2 aes-256-cbc -in <key> -out <key>.p8"
    ))]
    TripleDesEncrypted {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "This private key uses PBES1, which is obsolete. Convert it with: \
         openssl pkcs8 -topk8 -v2 aes-256-cbc -in <key> -out <key>.p8"
    ))]
    UnsupportedPbes1 {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "This private key was wrapped using scrypt, which is not supported. Convert it with: \
         openssl pkcs8 -topk8 -v2 aes-256-cbc -in <key> -out <key>.p8"
    ))]
    UnsupportedScrypt {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported PBKDF2 pseudo-random function: {prf}"))]
    UnsupportedPrf {
        prf: &'static str,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Unsupported private-key encryption cipher: {cipher}. Convert the key with: \
         openssl pkcs8 -topk8 -v2 aes-256-cbc -in <key> -out <key>.p8"
    ))]
    UnsupportedCipher {
        cipher: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Encrypted private key declares a zero PBKDF2 iteration count"))]
    ZeroIterations {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cryptographic operation failed while {operation}"))]
    Crypto {
        operation: String,
        source: aws_lc_rs::error::Unspecified,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cryptographic operation failed while {operation}"))]
    CryptoBackend {
        operation: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
pub(crate) mod test_support {
    use openssl::bn::BigNum;
    use openssl::ecdsa::EcdsaSig;
    use openssl::hash::MessageDigest;
    use openssl::pkey::{HasPublic, PKeyRef};
    use openssl::sign::Verifier;

    /// Verify a JWS signature with OpenSSL. `ES*` signatures are the raw
    /// `r || s` of RFC 7518 §3.4, re-encoded as the DER `ECDSA-Sig-Value`
    /// OpenSSL expects.
    pub(crate) fn verify_jws_signature<T: HasPublic>(
        public: &PKeyRef<T>,
        alg: &str,
        message: &[u8],
        signature: &[u8],
    ) -> bool {
        let (digest, der) = match alg {
            "RS256" => (MessageDigest::sha256(), signature.to_vec()),
            "ES256" | "ES384" | "ES512" => {
                let digest = match alg {
                    "ES256" => MessageDigest::sha256(),
                    "ES384" => MessageDigest::sha384(),
                    _ => MessageDigest::sha512(),
                };
                let (r, s) = signature.split_at(signature.len() / 2);
                let sig = EcdsaSig::from_private_components(
                    BigNum::from_slice(r).expect("r"),
                    BigNum::from_slice(s).expect("s"),
                )
                .expect("ecdsa sig");
                (digest, sig.to_der().expect("ecdsa sig der"))
            }
            other => panic!("unexpected JWS alg {other}"),
        };
        let mut verifier = Verifier::new(digest, public).expect("openssl verifier");
        verifier.update(message).expect("verifier update");
        verifier.verify(&der).expect("verify")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ec::{EcGroup, EcKey};
    use openssl::nid::Nid;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::symm::Cipher;

    const PASSPHRASE: &str = "correct horse battery staple";
    const MESSAGE: &[u8] = b"snowflake jwt signing input";

    /// A fresh 2048-bit RSA key, as OpenSSL would generate it.
    fn openssl_key() -> PKey<openssl::pkey::Private> {
        PKey::from_rsa(Rsa::generate(2048).expect("rsa keygen")).expect("pkey")
    }

    /// Sign `MESSAGE` with a key loaded by *our* code, then verify the
    /// signature with OpenSSL against the *original* key's public half.
    ///
    /// This is the assertion that matters. A decrypt that merely returns
    /// without error proves nothing -- it could yield the wrong bytes, or a
    /// structurally valid but different key. Verifying under OpenSSL with the
    /// original public key proves we recovered exactly the key the user has,
    /// and that it works for the production operation (RSA-SHA256 signing).
    fn assert_recovers(ours: &SigningKey, original: &PKey<openssl::pkey::Private>) {
        let sig = ours.sign(MESSAGE).expect("sign with recovered key");
        assert!(
            test_support::verify_jws_signature(original, ours.jwt_algorithm(), MESSAGE, &sig),
            "signature from the recovered key must verify under the original public key"
        );
    }

    /// AES-256-CBC is the format Snowflake's docs *should* recommend, and the
    /// one whose whole crypto path runs inside AWS-LC.
    #[test]
    fn aes256_encrypted_pkcs8_round_trips_as_pem() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        let loaded = load_signing_key(&pem, Some(PASSPHRASE)).expect("load AES-256-CBC PEM");
        assert_recovers(&loaded, &key);
    }

    /// The same key as bare DER. This is the Python connector's input shape,
    /// and it reaches a different branch than the armoured form: the envelope
    /// has to be recognised without a PEM label to go on.
    #[test]
    fn aes256_encrypted_pkcs8_round_trips_as_der() {
        let key = openssl_key();
        let der = key
            .private_key_to_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 der");
        let loaded = load_signing_key(&der, Some(PASSPHRASE)).expect("load AES-256-CBC DER");
        assert_recovers(&loaded, &key);
    }

    /// AES-192-CBC and AES-128-CBC are both legal PBES2 ciphers that
    /// `openssl pkcs8 -topk8 -v2 aes-192-cbc` / `-v2 aes-128-cbc` emit, so a
    /// key in either form loaded fine under the OpenSSL loader this replaced.
    /// The 192-bit arm was originally omitted on the mistaken belief that
    /// `aws_lc_rs::cipher` exposed no 192-bit key; it exposes `AES_192`, and
    /// CBC with a 128-bit IV is a valid context for it. Both are
    /// FIPS-approved and run wholly inside AWS-LC, so there was never a
    /// compliance reason to refuse them -- only a parity regression to cause.
    #[test]
    fn aes192_and_aes128_encrypted_pkcs8_round_trip() {
        for (label, cipher) in [
            ("AES-192-CBC", Cipher::aes_192_cbc()),
            ("AES-128-CBC", Cipher::aes_128_cbc()),
        ] {
            let key = openssl_key();
            let pem = key
                .private_key_to_pem_pkcs8_passphrase(cipher, PASSPHRASE.as_bytes())
                .unwrap_or_else(|e| panic!("{label} encrypted pkcs8 pem: {e}"));
            let loaded = load_signing_key(&pem, Some(PASSPHRASE))
                .unwrap_or_else(|e| panic!("load {label} PEM: {e}"));
            assert_recovers(&loaded, &key);
        }
    }

    /// `openssl pkcs8 -topk8 -v2 des3` writes PBES2 with 3DES-CBC. A standard
    /// build unwraps it. A `fips` build rejects it; see
    /// `des_ede3_encrypted_pkcs8_is_rejected_in_fips_builds`.
    #[cfg(not(feature = "fips"))]
    #[test]
    fn des_ede3_encrypted_pkcs8_round_trips() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        let loaded = load_signing_key(&pem, Some(PASSPHRASE)).expect("load 3DES PEM");
        assert_recovers(&loaded, &key);

        let der = key
            .private_key_to_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 der");
        let loaded = load_signing_key(&der, Some(PASSPHRASE)).expect("load 3DES DER");
        assert_recovers(&loaded, &key);
    }

    /// A `fips` build names the cipher and the build, and tells the caller
    /// how to re-encrypt the key. PEM and bare DER take the same path.
    #[cfg(feature = "fips")]
    #[test]
    fn des_ede3_encrypted_pkcs8_is_rejected_in_fips_builds() {
        let key = openssl_key();
        for encoded in [
            key.private_key_to_pem_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 pem"),
            key.private_key_to_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 der"),
        ] {
            let err = load_signing_key(&encoded, Some(PASSPHRASE)).expect_err("must be refused");
            assert!(matches!(err, PrivateKeyError::TripleDesEncrypted { .. }));
            assert!(
                err.to_string().contains("openssl pkcs8 -topk8"),
                "error must tell the user how to convert the key, got: {err}"
            );
            assert!(
                err.to_string().contains("standard builds"),
                "error must distinguish the build from the key, got: {err}"
            );
        }
    }

    #[test]
    fn unencrypted_pkcs8_round_trips() {
        let key = openssl_key();
        let pem = key.private_key_to_pem_pkcs8().expect("pkcs8 pem");
        let loaded = load_signing_key(&pem, None).expect("load unencrypted pkcs8 pem");
        assert_recovers(&loaded, &key);
    }

    /// The `-----BEGIN RSA PRIVATE KEY-----` shape, which the drivers have
    /// historically accepted alongside PKCS#8.
    #[test]
    fn unencrypted_pkcs1_round_trips() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa.private_key_to_pem().expect("pkcs1 pem");
        let key = PKey::from_rsa(rsa).expect("pkey");
        let loaded = load_signing_key(&pem, None).expect("load pkcs1");
        assert_recovers(&loaded, &key);
    }

    /// Bare DER with no PEM armour -- the Python connector's path.
    #[test]
    fn bare_der_round_trips() {
        let key = openssl_key();
        let der = key.private_key_to_pkcs8().expect("pkcs8 der");
        let loaded = load_signing_key(&der, None).expect("load bare DER");
        assert_recovers(&loaded, &key);
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        assert!(load_signing_key(&pem, Some("not the passphrase")).is_err());
    }

    /// An encrypted key with no passphrase must say *that*, rather than
    /// surfacing a confusing parse failure.
    #[test]
    fn missing_passphrase_reports_the_real_problem() {
        let key = openssl_key();
        // Both shapes must diagnose it the same way.
        for encoded in [
            key.private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 pem"),
            key.private_key_to_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 der"),
        ] {
            assert!(matches!(
                load_signing_key(&encoded, None),
                Err(PrivateKeyError::PassphraseRequired { .. })
            ));
        }
    }

    /// A passphrase on an unencrypted key is ignored, matching OpenSSL.
    #[test]
    fn passphrase_on_unencrypted_key_is_ignored() {
        let key = openssl_key();
        let pem = key.private_key_to_pem_pkcs8().expect("pkcs8 pem");
        let loaded =
            load_signing_key(&pem, Some(PASSPHRASE)).expect("load with spurious passphrase");
        assert_recovers(&loaded, &key);
    }

    /// Traditional encrypted PEM is the format `openssl rsa -aes256` and
    /// `openssl genrsa -aes256` produce, and the OpenSSL loader this module
    /// replaced read it. Standard builds keep that parity.
    ///
    /// Every cipher OpenSSL will emit here is covered, including the 3DES form
    /// from `-des3`, because the KDF derives a different key length for each
    /// and the IV width differs between AES (16) and 3DES (8) -- a mistake in
    /// either would surface as a wrong key rather than as a parse error.
    #[cfg(not(feature = "fips"))]
    #[test]
    fn legacy_encrypted_pem_round_trips() {
        for (label, cipher) in [
            ("AES-256-CBC", Cipher::aes_256_cbc()),
            ("AES-192-CBC", Cipher::aes_192_cbc()),
            ("AES-128-CBC", Cipher::aes_128_cbc()),
            ("DES-EDE3-CBC", Cipher::des_ede3_cbc()),
        ] {
            // `private_key_to_pem_passphrase` is an `Rsa` method and is what
            // emits the traditional format; `PKey` only offers the PKCS#8 one.
            let rsa = Rsa::generate(2048).expect("rsa keygen");
            let pem = rsa
                .private_key_to_pem_passphrase(cipher, PASSPHRASE.as_bytes())
                .unwrap_or_else(|e| panic!("{label} legacy encrypted pem: {e}"));
            assert!(
                String::from_utf8_lossy(&pem).contains("Proc-Type: 4,ENCRYPTED"),
                "{label}: OpenSSL did not produce the traditional format"
            );
            let original = PKey::from_rsa(rsa).expect("pkey");
            let loaded = load_signing_key(&pem, Some(PASSPHRASE))
                .unwrap_or_else(|e| panic!("load {label} legacy PEM: {e}"));
            assert_recovers(&loaded, &original);
        }
    }

    /// A wrong passphrase must fail rather than yield a mangled key: the KDF
    /// happily derives *a* key from any passphrase, so the only thing standing
    /// between a typo and a corrupt key is the padding check.
    #[cfg(not(feature = "fips"))]
    #[test]
    fn legacy_encrypted_pem_rejects_wrong_passphrase() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("legacy encrypted pem");
        assert!(load_signing_key(&pem, Some("not-the-passphrase")).is_err());
        assert!(matches!(
            load_signing_key(&pem, None),
            Err(PrivateKeyError::PassphraseRequired { .. })
        ));
    }

    /// FIPS builds cannot derive a key with MD5, so the same key is refused --
    /// with a message that says it is the build talking, not the key.
    #[cfg(feature = "fips")]
    #[test]
    fn legacy_encrypted_pem_is_rejected_in_fips_builds() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("legacy encrypted pem");
        let err = load_signing_key(&pem, Some(PASSPHRASE)).expect_err("must be refused");
        assert!(matches!(err, PrivateKeyError::LegacyEncryptedPem { .. }));
        assert!(
            err.to_string().contains("openssl pkcs8 -topk8"),
            "error must tell the user how to convert the key, got: {err}"
        );
        assert!(
            err.to_string().contains("standard builds"),
            "error must distinguish the build from the key, got: {err}"
        );
    }

    const SUPPORTED_CURVES: [(Nid, &str); 3] = [
        (Nid::X9_62_PRIME256V1, "ES256"),
        (Nid::SECP384R1, "ES384"),
        (Nid::SECP521R1, "ES512"),
    ];

    fn openssl_ec_key(nid: Nid) -> EcKey<openssl::pkey::Private> {
        let group = EcGroup::from_curve_name(nid).expect("ec group");
        EcKey::generate(&group).expect("ec keygen")
    }

    fn assert_ec_recovers(
        loaded: &SigningKey,
        original: &EcKey<openssl::pkey::Private>,
        expected_alg: &str,
    ) {
        assert_eq!(loaded.jwt_algorithm(), expected_alg);
        let original = PKey::from_ec_key(original.clone()).expect("pkey");
        assert_recovers(loaded, &original);
    }

    #[test]
    fn ec_pkcs8_round_trips_as_pem_and_der_on_each_curve() {
        for (nid, alg) in SUPPORTED_CURVES {
            let ec = openssl_ec_key(nid);
            let pkey = PKey::from_ec_key(ec.clone()).expect("pkey");
            let pem = pkey.private_key_to_pem_pkcs8().expect("pkcs8 pem");
            let der = pkey.private_key_to_pkcs8().expect("pkcs8 der");
            for encoded in [pem, der] {
                let loaded = load_signing_key(&encoded, None)
                    .unwrap_or_else(|e| panic!("load {alg} PKCS#8: {e}"));
                assert_ec_recovers(&loaded, &ec, alg);
            }
        }
    }

    /// `-----BEGIN EC PRIVATE KEY-----`, which `openssl ecparam -genkey` and
    /// `openssl ec` write, and the bare SEC1 DER Python's
    /// `PrivateFormat.TraditionalOpenSSL` produces.
    #[test]
    fn ec_sec1_round_trips_as_pem_and_der_on_each_curve() {
        for (nid, alg) in SUPPORTED_CURVES {
            let ec = openssl_ec_key(nid);
            let pem = ec.private_key_to_pem().expect("sec1 pem");
            assert!(String::from_utf8_lossy(&pem).starts_with("-----BEGIN EC PRIVATE KEY-----"));
            let der = ec.private_key_to_der().expect("sec1 der");
            assert_eq!(der_pem_label(&der).expect("sec1 label"), LABEL_SEC1);
            for encoded in [pem, der] {
                let loaded = load_signing_key(&encoded, None)
                    .unwrap_or_else(|e| panic!("load {alg} SEC1: {e}"));
                assert_ec_recovers(&loaded, &ec, alg);
            }
        }
    }

    #[test]
    fn ec_aes256_encrypted_pkcs8_round_trips_on_each_curve() {
        for (nid, alg) in SUPPORTED_CURVES {
            let ec = openssl_ec_key(nid);
            let pkey = PKey::from_ec_key(ec.clone()).expect("pkey");
            let pem = pkey
                .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 pem");
            let der = pkey
                .private_key_to_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
                .expect("encrypted pkcs8 der");
            for encoded in [pem, der] {
                let loaded = load_signing_key(&encoded, Some(PASSPHRASE))
                    .unwrap_or_else(|e| panic!("load {alg} encrypted PKCS#8: {e}"));
                assert_ec_recovers(&loaded, &ec, alg);
            }
        }
    }

    /// `openssl ec -aes256` writes traditional encrypted PEM under the
    /// `EC PRIVATE KEY` label.
    #[cfg(not(feature = "fips"))]
    #[test]
    fn ec_legacy_encrypted_pem_round_trips() {
        let ec = openssl_ec_key(Nid::X9_62_PRIME256V1);
        let pem = ec
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("legacy encrypted ec pem");
        assert!(String::from_utf8_lossy(&pem).contains("Proc-Type: 4,ENCRYPTED"));
        let loaded = load_signing_key(&pem, Some(PASSPHRASE)).expect("load legacy EC PEM");
        assert_ec_recovers(&loaded, &ec, "ES256");
    }

    #[test]
    fn ec_parameters_block_before_the_key_is_skipped() {
        let ec = openssl_ec_key(Nid::X9_62_PRIME256V1);
        let key_pem = String::from_utf8(ec.private_key_to_pem().expect("sec1 pem")).expect("utf8");
        let with_params = format!(
            "-----BEGIN EC PARAMETERS-----\nBggqhkjOPQMBBw==\n-----END EC PARAMETERS-----\n{key_pem}"
        );
        let loaded =
            load_signing_key(with_params.as_bytes(), None).expect("load key after EC PARAMETERS");
        assert_ec_recovers(&loaded, &ec, "ES256");
    }

    #[test]
    fn ec_key_on_unsupported_curve_is_rejected() {
        let ec = openssl_ec_key(Nid::SECP256K1);
        let pkcs8 = PKey::from_ec_key(ec.clone())
            .expect("pkey")
            .private_key_to_pem_pkcs8()
            .expect("pkcs8 pem");
        let err = load_signing_key(&pkcs8, None).expect_err("secp256k1 must be refused");
        assert!(
            matches!(&err, PrivateKeyError::UnsupportedEcCurve { curve, .. } if curve == "1.3.132.0.10"),
            "expected the unsupported curve to be named, got: {err}"
        );

        let sec1 = ec.private_key_to_pem().expect("sec1 pem");
        let err = load_signing_key(&sec1, None).expect_err("secp256k1 must be refused");
        assert!(
            matches!(&err, PrivateKeyError::KeyRejected { format, .. } if format == "SEC1"),
            "got: {err}"
        );
    }

    #[test]
    fn non_rsa_non_ec_pkcs8_key_is_rejected() {
        let pem = PKey::generate_ed25519()
            .expect("ed25519 keygen")
            .private_key_to_pem_pkcs8()
            .expect("pkcs8 pem");
        let err = load_signing_key(&pem, None).expect_err("ed25519 must be refused");
        assert!(
            matches!(&err, PrivateKeyError::UnsupportedKeyAlgorithm { algorithm, .. } if algorithm == "1.3.101.112"),
            "got: {err}"
        );
    }

    #[test]
    fn rsa_key_signs_rs256() {
        let key = openssl_key();
        let loaded = load_signing_key(&key.private_key_to_pem_pkcs8().expect("pem"), None)
            .expect("load rsa");
        assert_eq!(loaded.jwt_algorithm(), "RS256");
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(load_signing_key(b"not a key at all", None).is_err());
        assert!(load_signing_key(b"", None).is_err());
    }

    #[test]
    fn unsupported_pem_label_is_named() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
        assert!(matches!(
            load_signing_key(pem.as_bytes(), None),
            Err(PrivateKeyError::UnsupportedPemLabel { .. })
        ));
    }

    /// A mismatched END label is a framing error and must be reported as one.
    /// Without the check it parses: the body is taken verbatim and handed to
    /// the parser the *BEGIN* label selected, so the failure surfaces as a
    /// confusing "rejected as PKCS#1" much further down.
    #[test]
    fn mismatched_pem_end_label_is_rejected() {
        let key = openssl_key();
        let pem = String::from_utf8(key.private_key_to_pem_pkcs8().expect("pkcs8 pem"))
            .expect("pem is utf8");
        let mangled = pem.replace("-----END PRIVATE KEY-----", "-----END CERTIFICATE-----");
        let err = load_signing_key(mangled.as_bytes(), None).expect_err("must be refused");
        assert!(
            matches!(&err, PrivateKeyError::MalformedPem { detail, .. }
                if detail.contains("does not match END label")),
            "expected a framing error naming both labels, got: {err}"
        );
    }
}
