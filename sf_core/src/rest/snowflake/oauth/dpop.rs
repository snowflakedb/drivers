//! DPoP (RFC 9449) proof-of-possession helpers.
//!
//! ES256 P-256 keypair, proof JWT with `jti`/`htm`/`htu`/`iat` (and
//! optional `nonce` on `use_dpop_nonce` retry), `dpop_jkt` thumbprint on
//! the `/authorize` request, and a bundled access-token cache row. Only
//! JDBC has DPoP parity today among Snowflake drivers.
//!
//! Implementation notes:
//! - JWS signature is hand-built so we can attach the `jwk` header parameter
//!   (which the `jwt` crate's [`jwt::Header`] does not expose). The signing
//!   primitive is AWS-LC, the same module that carries TLS.
//! - `htu` deliberately strips the URL query string and fragment per
//!   RFC 9449 §4.3.
//! - JWK thumbprint is computed over the canonical RFC 7638 form:
//!   `{"crv":"P-256","kty":"EC","x":"…","y":"…"}` with lex-sorted keys,
//!   no whitespace.
//! - The signature is encoded in JOSE format (concatenated R||S, fixed
//!   32 bytes each). AWS-LC's `ECDSA_P256_SHA256_FIXED_SIGNING` emits exactly
//!   that, so there is no DER envelope to peel and no manual zero-padding.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use aws_lc_rs::encoding::AsBigEndian;
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::header::HeaderMap;
use snafu::{OptionExt, ResultExt};
use url::Url;
use uuid::Uuid;

use super::error::{DPoPJwkParseSnafu, DPoPProofGenerationSnafu, OAuthError};
use crate::sensitive::SensitiveString;

const DPOP_NONCE_HEADER: &str = "DPoP-Nonce";
/// Fixed width of a P-256 scalar or affine coordinate, in bytes. JWK requires
/// these octet strings be exactly the curve's field size (RFC 7518 §6.2.1.2),
/// and AWS-LC's SEC1 parsers expect the same.
const COORD_BYTES: usize = 32;
/// Length of an uncompressed SEC1 point: `0x04 || X || Y`.
const UNCOMPRESSED_POINT_BYTES: usize = 1 + 2 * COORD_BYTES;

/// A P-256 keypair used for DPoP proofs, backed by AWS-LC.
///
/// The keypair lives behind an `Arc` because `EcdsaKeyPair` is not `Clone`
/// (AWS-LC key objects are deliberately not copyable) while `DPoPContext`
/// needs an owned handle. Cloning a `DPoPKey` is therefore a refcount bump,
/// not a key copy.
#[derive(Clone)]
pub(crate) struct DPoPKey {
    key: Arc<EcdsaKeyPair>,
}

impl DPoPKey {
    /// Generate a fresh ES256 P-256 keypair.
    pub(crate) fn generate() -> Result<Self, OAuthError> {
        let key = EcdsaKeyPair::generate(&ECDSA_P256_SHA256_FIXED_SIGNING)
            .context(DPoPProofGenerationSnafu)?;
        Ok(Self { key: Arc::new(key) })
    }

    /// Recover a key previously serialized by [`DPoPKey::to_jwk_json`].
    /// Wired into the DPoP-bundled cache rehydration path (B.11) and the
    /// Snowflake-login DPoP signing path (C.5).
    pub(crate) fn from_jwk_json(json: &str) -> Result<Self, OAuthError> {
        let jwk: serde_json::Value = serde_json::from_str(json).map_err(|e| {
            DPoPJwkParseSnafu {
                reason: format!("invalid DPoP JWK JSON: {e}"),
            }
            .build()
        })?;
        let kty = jwk.get("kty").and_then(|v| v.as_str()).unwrap_or("");
        let crv = jwk.get("crv").and_then(|v| v.as_str()).unwrap_or("");
        if kty != "EC" || crv != "P-256" {
            return DPoPJwkParseSnafu {
                reason: format!("unsupported DPoP JWK: kty={kty} crv={crv}"),
            }
            .fail();
        }
        let d_b64 = jwk
            .get("d")
            .and_then(|v| v.as_str())
            .with_context(|| DPoPJwkParseSnafu {
                reason: "DPoP JWK is missing the private component (`d`)".to_string(),
            })?;
        let x_b64 = jwk
            .get("x")
            .and_then(|v| v.as_str())
            .with_context(|| DPoPJwkParseSnafu {
                reason: "DPoP JWK is missing `x`".to_string(),
            })?;
        let y_b64 = jwk
            .get("y")
            .and_then(|v| v.as_str())
            .with_context(|| DPoPJwkParseSnafu {
                reason: "DPoP JWK is missing `y`".to_string(),
            })?;
        let d = decode_b64url_coord(d_b64, "d")?;
        let x = decode_b64url_coord(x_b64, "x")?;
        let y = decode_b64url_coord(y_b64, "y")?;

        // AWS-LC takes the public half as an uncompressed SEC1 point rather
        // than affine coordinates, and cross-checks it against the private
        // scalar, so a JWK whose `d` and `x`/`y` disagree is rejected here
        // instead of producing a key that signs unverifiably.
        let mut point = Vec::with_capacity(UNCOMPRESSED_POINT_BYTES);
        point.push(0x04);
        point.extend_from_slice(&x);
        point.extend_from_slice(&y);

        let key = EcdsaKeyPair::from_private_key_and_public_key(
            &ECDSA_P256_SHA256_FIXED_SIGNING,
            &d,
            &point,
        )
        .map_err(|e| {
            DPoPJwkParseSnafu {
                reason: format!("DPoP JWK components did not form a valid P-256 key: {e}"),
            }
            .build()
        })?;
        Ok(Self { key: Arc::new(key) })
    }

    /// Serialize the key as a JWK including the private component, so it
    /// can be reused across the token-acquisition leg and the Snowflake
    /// login-request leg (the JDBC bundled-cache pattern).
    pub(crate) fn to_jwk_json(&self) -> Result<String, OAuthError> {
        let (x_b64, y_b64) = self.public_xy_b64()?;
        let d = self
            .key
            .private_key()
            .as_be_bytes()
            .context(DPoPProofGenerationSnafu)?;
        let d_b64 = URL_SAFE_NO_PAD.encode(d.as_ref());
        Ok(format!(
            r#"{{"crv":"P-256","d":"{d_b64}","kty":"EC","x":"{x_b64}","y":"{y_b64}"}}"#
        ))
    }

    /// Affine coordinates, sliced out of the uncompressed SEC1 point.
    ///
    /// AWS-LC hands back `0x04 || X || Y` with both halves already at the
    /// curve's fixed width, which is exactly what JWK wants -- so unlike the
    /// OpenSSL path there is no BigNum round-trip and no re-padding.
    fn public_xy_b64(&self) -> Result<(String, String), OAuthError> {
        let point = self.key.public_key().as_ref();
        if point.len() != UNCOMPRESSED_POINT_BYTES || point[0] != 0x04 {
            // Unreachable for a P-256 key from AWS-LC; asserted rather than
            // assumed because everything below is a fixed-offset slice.
            return DPoPJwkParseSnafu {
                reason: format!(
                    "expected a {UNCOMPRESSED_POINT_BYTES}-byte uncompressed P-256 point, \
                     got {} bytes",
                    point.len()
                ),
            }
            .fail();
        }
        let (x, y) = point[1..].split_at(COORD_BYTES);
        Ok((URL_SAFE_NO_PAD.encode(x), URL_SAFE_NO_PAD.encode(y)))
    }
}

/// Decode a base64url JWK component into a fixed-width big-endian scalar.
///
/// Left-pads short inputs. RFC 7518 §6.2.1.2 requires full-width octet
/// strings, but an implementation that encodes the value as a minimal integer
/// drops leading zero bytes -- which happens for roughly 1 in 256 keys per
/// component. The OpenSSL path absorbed this silently via `BigNum`; AWS-LC's
/// SEC1 parsers want exact width, so the padding has to be explicit or those
/// keys would fail to rehydrate from cache.
fn decode_b64url_coord(s: &str, field: &str) -> Result<[u8; COORD_BYTES], OAuthError> {
    let bytes = URL_SAFE_NO_PAD.decode(s.as_bytes()).map_err(|e| {
        DPoPJwkParseSnafu {
            reason: format!("invalid base64url in DPoP JWK component `{field}`: {e}"),
        }
        .build()
    })?;
    if bytes.len() > COORD_BYTES {
        return DPoPJwkParseSnafu {
            reason: format!(
                "DPoP JWK component `{field}` is {} bytes, more than the {COORD_BYTES} a P-256 \
                 value can hold",
                bytes.len()
            ),
        }
        .fail();
    }
    let mut out = [0u8; COORD_BYTES];
    out[COORD_BYTES - bytes.len()..].copy_from_slice(&bytes);
    Ok(out)
}

/// Compute the canonical RFC 7638 JWK SHA-256 thumbprint for the public
/// portion of `key`. Output is base64url no-padding.
pub(crate) fn jwk_thumbprint(key: &DPoPKey) -> Result<String, OAuthError> {
    let (x, y) = key.public_xy_b64()?;
    let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
    let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, canonical.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(digest.as_ref()))
}

/// Build a DPoP proof JWT for `(method, url)` and optional server `nonce`.
///
/// Per RFC 9449 §4.3 the `htu` claim is the request URI **without** query
/// string or fragment. The proof header carries the public JWK so the
/// resource server can verify the signature without separate key lookup.
pub(crate) fn proof_jwt(
    key: &DPoPKey,
    method: &str,
    url: &Url,
    nonce: Option<&str>,
) -> Result<SensitiveString, OAuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (x_b64, y_b64) = key.public_xy_b64()?;

    let header = format!(
        r#"{{"alg":"ES256","jwk":{{"crv":"P-256","kty":"EC","x":"{x_b64}","y":"{y_b64}"}},"typ":"dpop+jwt"}}"#
    );

    let htu = htu_value(url);
    let jti = Uuid::new_v4().to_string();
    let claims = match nonce {
        Some(n) => {
            let n_escaped = json_escape(n);
            format!(
                r#"{{"htm":"{method}","htu":"{htu}","iat":{now},"jti":"{jti}","nonce":"{n_escaped}"}}"#
            )
        }
        None => format!(r#"{{"htm":"{method}","htu":"{htu}","iat":{now},"jti":"{jti}"}}"#),
    };

    let header_b64 = URL_SAFE_NO_PAD.encode(header.as_bytes());
    let claims_b64 = URL_SAFE_NO_PAD.encode(claims.as_bytes());

    let signing_input = format!("{header_b64}.{claims_b64}");
    // `sign` hashes the message itself. Deliberately not `sign_digest` with a
    // pre-computed hash: AWS-LC marks that path as not FIPS-approved, so a
    // `fips-tls` build must hand over the message and let the module hash it.
    let sig = key
        .key
        .sign(&SystemRandom::new(), signing_input.as_bytes())
        .context(DPoPProofGenerationSnafu)?;
    // `ECDSA_P256_SHA256_FIXED_SIGNING` already emits R||S at 32 bytes each.
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.as_ref());

    Ok(SensitiveString::from(format!(
        "{header_b64}.{claims_b64}.{sig_b64}"
    )))
}

/// Compute the `htu` claim per RFC 9449 §4.3 — strip query and fragment.
fn htu_value(url: &Url) -> String {
    let mut u = url.clone();
    u.set_query(None);
    u.set_fragment(None);
    u.to_string()
}

/// Minimal JSON string escape sufficient for the IdP-supplied nonce.
/// We escape `\\`, `"`, and ASCII control characters; everything else is
/// passed through. Nonces are conventionally base64url, so the escape
/// path almost never fires — we keep it for defense in depth.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// If the IdP responded with an `error == "use_dpop_nonce"` body and a
/// `DPoP-Nonce` header, return the nonce so the caller can retry once
/// with the nonce embedded in the proof (mirrors JDBC
/// `RestRequest.checkForDPoPNonceError`).
pub(crate) fn check_use_dpop_nonce(headers: &HeaderMap, body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let error = parsed.get("error").and_then(|v| v.as_str())?;
    if error != "use_dpop_nonce" {
        return None;
    }
    let nonce = headers
        .get(DPOP_NONCE_HEADER)
        .or_else(|| headers.get("dpop-nonce"))?
        .to_str()
        .ok()?;
    Some(nonce.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn split_jwt(jwt: &str) -> (Value, Value, Vec<u8>) {
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "DPoP JWT must have exactly three segments");
        let header: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(parts[0].as_bytes())
                .expect("header b64"),
        )
        .expect("header json");
        let claims: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(parts[1].as_bytes())
                .expect("claims b64"),
        )
        .expect("claims json");
        let sig = URL_SAFE_NO_PAD
            .decode(parts[2].as_bytes())
            .expect("sig b64");
        (header, claims, sig)
    }

    #[test]
    fn generated_key_round_trips_through_jwk_json() {
        let k = DPoPKey::generate().expect("generate");
        let json = k.to_jwk_json().expect("to_jwk_json");
        let k2 = DPoPKey::from_jwk_json(&json).expect("from_jwk_json");
        assert_eq!(
            jwk_thumbprint(&k).unwrap(),
            jwk_thumbprint(&k2).unwrap(),
            "thumbprint must be stable across roundtrip"
        );
        let json_again = k2.to_jwk_json().unwrap();
        assert_eq!(json, json_again);
    }

    #[test]
    fn jwk_thumbprint_is_stable_for_same_key() {
        let k = DPoPKey::generate().unwrap();
        let a = jwk_thumbprint(&k).unwrap();
        let b = jwk_thumbprint(&k).unwrap();
        assert_eq!(a, b);
        assert!(!a.contains('='));
    }

    #[test]
    fn jwk_thumbprint_differs_across_keys() {
        let a = jwk_thumbprint(&DPoPKey::generate().unwrap()).unwrap();
        let b = jwk_thumbprint(&DPoPKey::generate().unwrap()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn proof_jwt_has_required_header_and_claims() {
        let key = DPoPKey::generate().unwrap();
        let url = Url::parse("https://idp.example.com/oauth/token-request").unwrap();
        let jwt = proof_jwt(&key, "POST", &url, None).unwrap();
        let (header, claims, sig) = split_jwt(jwt.reveal());
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["typ"], "dpop+jwt");
        assert_eq!(header["jwk"]["crv"], "P-256");
        assert_eq!(header["jwk"]["kty"], "EC");
        assert!(header["jwk"]["x"].as_str().unwrap().len() >= 42);
        assert!(header["jwk"]["y"].as_str().unwrap().len() >= 42);
        assert!(header["jwk"].get("d").is_none(), "private component leaked");
        assert_eq!(claims["htm"], "POST");
        assert_eq!(claims["htu"], url.as_str());
        assert!(claims["iat"].is_u64());
        assert!(!claims["jti"].as_str().unwrap().is_empty());
        assert_eq!(
            sig.len(),
            64,
            "ES256 JOSE signature must be exactly 64 bytes"
        );
    }

    #[test]
    fn proof_jwt_strips_query_and_fragment_from_htu() {
        let key = DPoPKey::generate().unwrap();
        let url = Url::parse("https://x.com/path?q=1#frag").unwrap();
        let jwt = proof_jwt(&key, "POST", &url, None).unwrap();
        let (_h, claims, _) = split_jwt(jwt.reveal());
        assert_eq!(claims["htu"], "https://x.com/path");
    }

    #[test]
    fn proof_jwt_with_nonce_includes_nonce_claim() {
        let key = DPoPKey::generate().unwrap();
        let url = Url::parse("https://idp.example.com/oauth/token-request").unwrap();
        let jwt = proof_jwt(&key, "POST", &url, Some("abc123")).unwrap();
        let (_h, claims, _) = split_jwt(jwt.reveal());
        assert_eq!(claims["nonce"], "abc123");
    }

    #[test]
    fn check_use_dpop_nonce_extracts_nonce_when_signaled() {
        let body = r#"{"error":"use_dpop_nonce","error_description":"DPoP nonce required"}"#;
        let mut headers = HeaderMap::new();
        headers.insert("DPoP-Nonce", "server-nonce-123".parse().unwrap());
        let n = check_use_dpop_nonce(&headers, body).expect("nonce found");
        assert_eq!(n, "server-nonce-123");
    }

    #[test]
    fn check_use_dpop_nonce_is_none_for_unrelated_errors() {
        let body = r#"{"error":"invalid_request"}"#;
        let mut headers = HeaderMap::new();
        headers.insert("DPoP-Nonce", "x".parse().unwrap());
        assert!(check_use_dpop_nonce(&headers, body).is_none());
    }

    #[test]
    fn check_use_dpop_nonce_is_none_when_header_missing() {
        let body = r#"{"error":"use_dpop_nonce"}"#;
        let headers = HeaderMap::new();
        assert!(check_use_dpop_nonce(&headers, body).is_none());
    }

    /// Sign with AWS-LC, verify with OpenSSL, using only the public JWK the
    /// proof itself carries.
    ///
    /// Two properties at once, both of which a resource server depends on:
    /// the signature is valid under an independent implementation (not just
    /// self-consistent with the signer), and the `jwk` header advertises the
    /// coordinates that actually correspond to the signing key. Verifying
    /// against `key`'s internal handle would prove neither.
    #[test]
    fn proof_jwt_signature_verifies_against_published_jwk() {
        use openssl::bn::BigNum;
        use openssl::ec::{EcGroup, EcKey};
        use openssl::ecdsa::EcdsaSig;
        use openssl::nid::Nid;

        let key = DPoPKey::generate().unwrap();
        let url = Url::parse("https://idp.example.com/oauth/token-request").unwrap();
        let jwt = proof_jwt(&key, "POST", &url, None).unwrap();
        let parts: Vec<&str> = jwt.reveal().split('.').collect();

        // Rebuild the verifying key from the proof's own `jwk` header.
        let (header, _, _) = split_jwt(jwt.reveal());
        let jwk = &header["jwk"];
        let x = URL_SAFE_NO_PAD
            .decode(jwk["x"].as_str().expect("jwk x"))
            .unwrap();
        let y = URL_SAFE_NO_PAD
            .decode(jwk["y"].as_str().expect("jwk y"))
            .unwrap();
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let public = EcKey::from_public_key_affine_coordinates(
            &group,
            &BigNum::from_slice(&x).unwrap(),
            &BigNum::from_slice(&y).unwrap(),
        )
        .expect("jwk coordinates form a valid P-256 point");

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let digest = openssl::sha::sha256(signing_input.as_bytes());
        let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        assert_eq!(
            sig_bytes.len(),
            2 * COORD_BYTES,
            "JOSE signature must be fixed-width R||S, not a DER envelope"
        );
        let (r_bytes, s_bytes) = sig_bytes.split_at(COORD_BYTES);
        let ecdsa = EcdsaSig::from_private_components(
            BigNum::from_slice(r_bytes).unwrap(),
            BigNum::from_slice(s_bytes).unwrap(),
        )
        .unwrap();
        assert!(
            ecdsa.verify(&digest, &public).unwrap(),
            "AWS-LC signature should verify under OpenSSL using the published JWK"
        );
    }

    /// A JWK component whose leading byte is zero may be encoded at less than
    /// the curve's full width by implementations that treat it as a minimal
    /// integer. The OpenSSL path absorbed that via `BigNum`; AWS-LC's SEC1
    /// parsers need exact width, so `decode_b64url_coord` left-pads.
    ///
    /// Without the padding, roughly 1 in 256 cached DPoP keys per component
    /// would fail to rehydrate.
    #[test]
    fn short_jwk_components_are_left_padded() {
        let padded = decode_b64url_coord(&URL_SAFE_NO_PAD.encode([0xAB, 0xCD]), "x").unwrap();
        let mut expected = [0u8; COORD_BYTES];
        expected[COORD_BYTES - 2] = 0xAB;
        expected[COORD_BYTES - 1] = 0xCD;
        assert_eq!(padded, expected, "short components should be left-padded");
    }

    /// An over-wide component is a malformed JWK, not something to truncate.
    #[test]
    fn oversized_jwk_components_are_rejected() {
        let too_long = URL_SAFE_NO_PAD.encode([0u8; COORD_BYTES + 1]);
        assert!(
            decode_b64url_coord(&too_long, "x").is_err(),
            "a component wider than the curve must be rejected"
        );
    }

    /// The cache round-trip must preserve the key, including the private
    /// scalar: the token leg and the login leg have to sign with the same key.
    #[test]
    fn jwk_json_round_trip_preserves_the_key() {
        let key = DPoPKey::generate().unwrap();
        let json = key.to_jwk_json().unwrap();
        let restored = DPoPKey::from_jwk_json(&json).unwrap();
        assert_eq!(
            json,
            restored.to_jwk_json().unwrap(),
            "rehydrated key should serialize identically"
        );
        assert_eq!(
            jwk_thumbprint(&key).unwrap(),
            jwk_thumbprint(&restored).unwrap(),
            "thumbprint must survive the cache round-trip"
        );
    }

    /// A JWK whose public half does not match its private scalar must be
    /// rejected rather than producing a key that signs unverifiably. AWS-LC
    /// cross-checks the components; OpenSSL's `check_key` did the same.
    #[test]
    fn jwk_with_mismatched_public_half_is_rejected() {
        let a = DPoPKey::generate().unwrap();
        let b = DPoPKey::generate().unwrap();
        let (a_x, _) = a.public_xy_b64().unwrap();
        let b_jwk: Value = serde_json::from_str(&b.to_jwk_json().unwrap()).unwrap();
        // b's private scalar and y, but a's x.
        let mismatched = format!(
            r#"{{"crv":"P-256","d":"{}","kty":"EC","x":"{a_x}","y":"{}"}}"#,
            b_jwk["d"].as_str().unwrap(),
            b_jwk["y"].as_str().unwrap()
        );
        assert!(
            DPoPKey::from_jwk_json(&mismatched).is_err(),
            "mismatched private/public components must be rejected"
        );
    }
}
