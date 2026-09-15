//! RSA private-key loading: PEM/DER in, an AWS-LC signing key out.
//!
//! Replaces OpenSSL's `Rsa::private_key_from_pem[_passphrase]`. Handles the
//! three shapes the drivers accept -- unencrypted PKCS#8, unencrypted PKCS#1,
//! and PBES2-encrypted PKCS#8 -- and produces an
//! [`aws_lc_rs::signature::RsaKeyPair`].
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
//! - **3DES-CBC decryption** runs in the `des` crate, outside the module. This
//!   is the one exception, and it is exact rather than grudging: 3DES is not an
//!   approved algorithm in *any* validated module, so routing it through AWS-LC
//!   would not make it approved. Outside is where it honestly belongs.
//!
//! # Why 3DES has to be supported at all
//!
//! Snowflake's own key-pair documentation gives exactly one command for
//! generating an encrypted private key:
//!
//! ```text
//! openssl genrsa 2048 | openssl pkcs8 -topk8 -v2 des3 -inform PEM -out rsa_key.p8
//! ```
//!
//! `-v2 des3` is PBES2 + 3DES, so essentially every encrypted key in the field
//! is 3DES-wrapped. Rejecting them would break the documented happy path, and
//! encrypted keys are table-stakes parity (JDBC, ODBC, Node.js, .NET and Python
//! all support them).
//!
//! **That documentation should be changed to `-v2 aes-256-cbc`.** Under NIST
//! SP 800-131A Rev. 2, three-key TDEA *encryption* and *key wrapping* have been
//! **disallowed since 2023-12-31**, so the documented command has users apply a
//! disallowed algorithm to protect their key. That is a problem with the
//! key-generation step, not with this code: what we do here is *unwrapping*,
//! and the same publication states plainly that "Decryption using three-key
//! TDEA is allowed for legacy use" and "Key unwrapping using three-key TDEA is
//! allowed for legacy use". So reading these keys stays permissible
//! indefinitely, while creating new ones is not.
//!
//! Legacy use is not free, though: SP 800-131A also says that it "require[s]
//! that the user accept some risk that increases over time", and that judgement
//! belongs to the organisation. That acceptance needs to be recorded in the
//! compliance exception list rather than implied by this module existing.
//!
//! Behaviour is identical with and without `fips-tls`. There is deliberately no
//! feature gate that rejects 3DES in FIPS builds: NIST permits the unwrap, so a
//! gate would break documented, supported keys for no regulatory reason.
//!
//! # The one place builds differ: traditional encrypted PEM
//!
//! `-----BEGIN RSA PRIVATE KEY-----` with `Proc-Type: 4,ENCRYPTED` -- what
//! `openssl rsa -aes256` and `openssl genrsa -aes256` emit -- is read in
//! standard builds and refused in `fips-tls` builds. See [`legacy_pem`].
//!
//! The gate is on the KDF, not the cipher. The format derives its key with
//! OpenSSL's `EVP_BytesToKey`, which is MD5-based, and MD5 is definitional to
//! the format rather than a parameter of it: there is no version of this that
//! runs inside AWS-LC, which exposes no MD5 at all. So unlike 3DES -- where
//! SP 800-131A permits the unwrap and the KDF is already approved PBKDF2 --
//! there is no reading of the rules under which a FIPS build may do this.
//!
//! Refusing it *everywhere* was the earlier behaviour and was the wrong trade:
//! it dropped a working format from every shipped artifact in order to protect
//! a claim only the FIPS artifact makes. A standard build asserts no FIPS
//! compliance, so nothing about it is compromised by using MD5 to open a key
//! its owner already has. Recorded as F16 in the compliance plan.

use aws_lc_rs::cipher::{
    AES_128, AES_192, AES_256, DecryptionContext, PaddedBlockDecryptingKey, UnboundCipherKey,
};
use aws_lc_rs::iv::{FixedLength, IV_LEN_128_BIT};
use aws_lc_rs::pbkdf2;
use aws_lc_rs::signature::RsaKeyPair;
use pkcs8::pkcs5::{self, pbes2};
use snafu::{Location, OptionExt, ResultExt, Snafu};

use crate::sensitive::Sensitive;

/// PEM labels we accept, in the forms the drivers have historically seen.
const LABEL_PKCS8: &str = "PRIVATE KEY";
const LABEL_PKCS8_ENCRYPTED: &str = "ENCRYPTED PRIVATE KEY";
const LABEL_PKCS1: &str = "RSA PRIVATE KEY";

/// Load an RSA signing key from PEM or DER.
///
/// `passphrase` is required for an encrypted key and ignored for an
/// unencrypted one, matching OpenSSL's behaviour.
pub(crate) fn load_rsa_key(
    key: &[u8],
    passphrase: Option<&str>,
) -> Result<RsaKeyPair, PrivateKeyError> {
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
/// Split by build, and this is the only place in the module where that changes
/// which keys load rather than only which module does the work. See
/// [`legacy_pem`] for why MD5 makes that unavoidable.
#[cfg(not(feature = "fips-tls"))]
fn from_legacy_pem(
    label: &str,
    dek_info: &str,
    body: &[u8],
    passphrase: Option<&str>,
) -> Result<RsaKeyPair, PrivateKeyError> {
    let passphrase = passphrase.with_context(|| PassphraseRequiredSnafu)?;
    let der = legacy_pem::decrypt(dek_info, body, passphrase)?;
    // The passphrase is spent; the plaintext is an ordinary unencrypted body.
    from_labelled_der(label, der.reveal(), None)
}

#[cfg(feature = "fips-tls")]
fn from_legacy_pem(
    _label: &str,
    _dek_info: &str,
    _body: &[u8],
    _passphrase: Option<&str>,
) -> Result<RsaKeyPair, PrivateKeyError> {
    LegacyEncryptedPemSnafu.fail()
}

/// The PEM label matching `der`'s actual format, validating it along the way.
///
/// Lets the config layer armour raw DER without guessing: a PKCS#1 body
/// labelled `PRIVATE KEY` would be handed to the PKCS#8 parser and rejected,
/// and an encrypted envelope labelled the same way would never reach the
/// decryption path at all.
///
/// The parse is deliberately repeated later by [`load_rsa_key`]. Validating
/// here is what keeps a malformed key an immediate, precise configuration
/// error instead of a confusing failure at first login.
pub(crate) fn der_pem_label(der: &[u8]) -> Result<&'static str, PrivateKeyError> {
    if pkcs8::EncryptedPrivateKeyInfo::try_from(der).is_ok() {
        return Ok(LABEL_PKCS8_ENCRYPTED);
    }
    if RsaKeyPair::from_pkcs8(der).is_ok() {
        return Ok(LABEL_PKCS8);
    }
    RsaKeyPair::from_der(der)
        .map(|_| LABEL_PKCS1)
        .context(KeyRejectedSnafu {
            format: "DER (tried PKCS#8 then PKCS#1)",
        })
}

fn from_labelled_der(
    label: &str,
    der: &[u8],
    passphrase: Option<&str>,
) -> Result<RsaKeyPair, PrivateKeyError> {
    match label {
        LABEL_PKCS8 => RsaKeyPair::from_pkcs8(der).context(KeyRejectedSnafu { format: "PKCS#8" }),
        LABEL_PKCS1 => RsaKeyPair::from_der(der).context(KeyRejectedSnafu { format: "PKCS#1" }),
        LABEL_PKCS8_ENCRYPTED => unwrap_encrypted(der, passphrase),
        other => UnsupportedPemLabelSnafu {
            label: other.to_string(),
        }
        .fail(),
    }
}

fn from_bare_der(der: &[u8], passphrase: Option<&str>) -> Result<RsaKeyPair, PrivateKeyError> {
    // Encrypted first. `EncryptedPrivateKeyInfo` is a distinctive shape, so
    // testing for it up front means a missing passphrase is reported as
    // exactly that, rather than as a generic "key rejected" from the plain
    // parsers failing on an envelope they were never given a chance to open.
    if pkcs8::EncryptedPrivateKeyInfo::try_from(der).is_ok() {
        return unwrap_encrypted(der, passphrase);
    }
    RsaKeyPair::from_pkcs8(der)
        .or_else(|_| RsaKeyPair::from_der(der))
        .context(KeyRejectedSnafu {
            format: "DER (tried PKCS#8 then PKCS#1)",
        })
}

fn unwrap_encrypted(der: &[u8], passphrase: Option<&str>) -> Result<RsaKeyPair, PrivateKeyError> {
    let passphrase = passphrase.with_context(|| PassphraseRequiredSnafu)?;
    let plaintext = decrypt_pkcs8(der, passphrase)?;
    RsaKeyPair::from_pkcs8(plaintext.reveal()).context(KeyRejectedSnafu {
        format: "decrypted PKCS#8",
    })
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
        pbes2::EncryptionScheme::DesEde3Cbc { iv } => des_ede3_cbc_decrypt(key, iv, ciphertext),
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

/// 3DES-CBC/PKCS#7, outside the validated module. See the module docs: this is
/// legacy-use decryption of a non-approved algorithm, which no module would
/// perform as an approved service anyway.
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
/// `fips-tls` build therefore cannot read these keys and says so, while a
/// standard build makes no FIPS claim and keeps the compatibility the OpenSSL
/// loader had.
///
/// This is the one place where build flags change which *keys load*, rather
/// than only which module does the work. It is a deliberate exception,
/// recorded as F16 in the compliance plan: the alternative was dropping a
/// working format from every shipped artifact to protect a claim only one of
/// them makes.
///
/// Note what is *not* gated: the bulk decryption still runs in AWS-LC for AES
/// (and in `des` for 3DES, as everywhere else in this module). Only the key
/// derivation is MD5.
#[cfg(not(feature = "fips-tls"))]
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

    /// Only reachable in `fips-tls` builds; standard builds read these keys.
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
mod tests {
    use super::*;
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::sign::Verifier;
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
    fn assert_recovers(ours: &RsaKeyPair, original: &PKey<openssl::pkey::Private>) {
        let mut sig = vec![0u8; ours.public_modulus_len()];
        ours.sign(
            &aws_lc_rs::signature::RSA_PKCS1_SHA256,
            &aws_lc_rs::rand::SystemRandom::new(),
            MESSAGE,
            &mut sig,
        )
        .expect("sign with recovered key");

        let mut verifier =
            Verifier::new(MessageDigest::sha256(), original).expect("openssl verifier");
        verifier.update(MESSAGE).expect("verifier update");
        assert!(
            verifier.verify(&sig).expect("verify"),
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
        let loaded = load_rsa_key(&pem, Some(PASSPHRASE)).expect("load AES-256-CBC PEM");
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
        let loaded = load_rsa_key(&der, Some(PASSPHRASE)).expect("load AES-256-CBC DER");
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
            let loaded = load_rsa_key(&pem, Some(PASSPHRASE))
                .unwrap_or_else(|e| panic!("load {label} PEM: {e}"));
            assert_recovers(&loaded, &key);
        }
    }

    /// 3DES is what `openssl pkcs8 -topk8 -v2 des3` produces -- the command
    /// Snowflake documents -- so this is the format most encrypted keys in the
    /// field actually use. Unwrapping it is NIST-permitted legacy use; see the
    /// module docs.
    #[test]
    fn des_ede3_encrypted_pkcs8_round_trips() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        let loaded = load_rsa_key(&pem, Some(PASSPHRASE)).expect("load 3DES PEM");
        assert_recovers(&loaded, &key);

        let der = key
            .private_key_to_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 der");
        let loaded = load_rsa_key(&der, Some(PASSPHRASE)).expect("load 3DES DER");
        assert_recovers(&loaded, &key);
    }

    /// Behaviour must not differ between FIPS and non-FIPS builds: there is no
    /// feature gate rejecting 3DES, because NIST permits the unwrap.
    #[test]
    fn des_ede3_is_accepted_regardless_of_fips_feature() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::des_ede3_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        assert!(
            load_rsa_key(&pem, Some(PASSPHRASE)).is_ok(),
            "3DES unwrapping is permitted legacy use and must work in every build"
        );
    }

    #[test]
    fn unencrypted_pkcs8_round_trips() {
        let key = openssl_key();
        let pem = key.private_key_to_pem_pkcs8().expect("pkcs8 pem");
        let loaded = load_rsa_key(&pem, None).expect("load unencrypted pkcs8 pem");
        assert_recovers(&loaded, &key);
    }

    /// The `-----BEGIN RSA PRIVATE KEY-----` shape, which the drivers have
    /// historically accepted alongside PKCS#8.
    #[test]
    fn unencrypted_pkcs1_round_trips() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa.private_key_to_pem().expect("pkcs1 pem");
        let key = PKey::from_rsa(rsa).expect("pkey");
        let loaded = load_rsa_key(&pem, None).expect("load pkcs1");
        assert_recovers(&loaded, &key);
    }

    /// Bare DER with no PEM armour -- the Python connector's path.
    #[test]
    fn bare_der_round_trips() {
        let key = openssl_key();
        let der = key.private_key_to_pkcs8().expect("pkcs8 der");
        let loaded = load_rsa_key(&der, None).expect("load bare DER");
        assert_recovers(&loaded, &key);
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let key = openssl_key();
        let pem = key
            .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("encrypted pkcs8 pem");
        assert!(load_rsa_key(&pem, Some("not the passphrase")).is_err());
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
                load_rsa_key(&encoded, None),
                Err(PrivateKeyError::PassphraseRequired { .. })
            ));
        }
    }

    /// A passphrase on an unencrypted key is ignored, matching OpenSSL.
    #[test]
    fn passphrase_on_unencrypted_key_is_ignored() {
        let key = openssl_key();
        let pem = key.private_key_to_pem_pkcs8().expect("pkcs8 pem");
        let loaded = load_rsa_key(&pem, Some(PASSPHRASE)).expect("load with spurious passphrase");
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
    #[cfg(not(feature = "fips-tls"))]
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
            let loaded = load_rsa_key(&pem, Some(PASSPHRASE))
                .unwrap_or_else(|e| panic!("load {label} legacy PEM: {e}"));
            assert_recovers(&loaded, &original);
        }
    }

    /// A wrong passphrase must fail rather than yield a mangled key: the KDF
    /// happily derives *a* key from any passphrase, so the only thing standing
    /// between a typo and a corrupt key is the padding check.
    #[cfg(not(feature = "fips-tls"))]
    #[test]
    fn legacy_encrypted_pem_rejects_wrong_passphrase() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("legacy encrypted pem");
        assert!(load_rsa_key(&pem, Some("not-the-passphrase")).is_err());
        assert!(matches!(
            load_rsa_key(&pem, None),
            Err(PrivateKeyError::PassphraseRequired { .. })
        ));
    }

    /// FIPS builds cannot derive a key with MD5, so the same key is refused --
    /// with a message that says it is the build talking, not the key.
    #[cfg(feature = "fips-tls")]
    #[test]
    fn legacy_encrypted_pem_is_rejected_in_fips_builds() {
        let rsa = Rsa::generate(2048).expect("rsa keygen");
        let pem = rsa
            .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), PASSPHRASE.as_bytes())
            .expect("legacy encrypted pem");
        let err = load_rsa_key(&pem, Some(PASSPHRASE)).expect_err("must be refused");
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

    #[test]
    fn garbage_is_rejected() {
        assert!(load_rsa_key(b"not a key at all", None).is_err());
        assert!(load_rsa_key(b"", None).is_err());
    }

    #[test]
    fn unsupported_pem_label_is_named() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
        assert!(matches!(
            load_rsa_key(pem.as_bytes(), None),
            Err(PrivateKeyError::UnsupportedPemLabel { .. })
        ));
    }
}
