//! DPoP (RFC 9449) proof-of-possession helpers.
//!
//! ES256 P-256 keypair, proof JWT with `jti`/`htm`/`htu`/`iat` (and
//! optional `nonce` on `use_dpop_nonce` retry), `dpop_jkt` thumbprint on
//! the `/authorize` request, and a bundled access-token cache row. Only
//! JDBC has parity today (analysis_feature_oauth.md §5).
//!
//! Implementation notes:
//! - JWS signature is hand-built so we can attach the `jwk` header parameter
//!   (which the `jwt` crate's [`jwt::Header`] does not expose). We still
//!   share the openssl-backed signing primitive with `sf_core::auth`.
//! - `htu` deliberately strips the URL query string and fragment per
//!   RFC 9449 §4.3 (also gotcha #2 in `analysis_feature_oauth.md` §14).
//! - JWK thumbprint is computed over the canonical RFC 7638 form:
//!   `{"crv":"P-256","kty":"EC","x":"…","y":"…"}` with lex-sorted keys,
//!   no whitespace.
//! - The signature is encoded in JOSE format (concatenated R||S, fixed
//!   32 bytes each — left-padded with zeros) so verifiers don't have to
//!   peel a DER envelope.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openssl::bn::{BigNum, BigNumContext};
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::nid::Nid;
use openssl::pkey::Private;
use openssl::sha::Sha256;
use reqwest::header::HeaderMap;
use snafu::ResultExt;
use url::Url;
use uuid::Uuid;

use super::error::{DPoPProofGenerationSnafu, OAuthError, TokenResponseDecodeSnafu};
use crate::sensitive::SensitiveString;

const DPOP_NONCE_HEADER: &str = "DPoP-Nonce";
const COORD_BYTES: i32 = 32;

/// Wrapper around an openssl P-256 EC private key used for DPoP proofs.
pub(crate) struct DPoPKey {
    key: EcKey<Private>,
}

impl DPoPKey {
    /// Generate a fresh ES256 P-256 keypair.
    pub(crate) fn generate() -> Result<Self, OAuthError> {
        let group =
            EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).context(DPoPProofGenerationSnafu)?;
        let key = EcKey::generate(&group).context(DPoPProofGenerationSnafu)?;
        Ok(Self { key })
    }

    /// Recover a key previously serialized by [`DPoPKey::to_jwk_json`].
    /// Reserved for the DPoP-bundled cache rehydration path that step 2.4
    /// will wire into the login retry loop.
    #[allow(dead_code)]
    pub(crate) fn from_jwk_json(json: &str) -> Result<Self, OAuthError> {
        let jwk: serde_json::Value =
            serde_json::from_str(json).context(TokenResponseDecodeSnafu)?;
        let kty = jwk.get("kty").and_then(|v| v.as_str()).unwrap_or("");
        let crv = jwk.get("crv").and_then(|v| v.as_str()).unwrap_or("");
        if kty != "EC" || crv != "P-256" {
            return Err(OAuthError::Internal {
                source: format!("unsupported DPoP JWK: kty={kty} crv={crv}").into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        let d_b64 = jwk
            .get("d")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OAuthError::Internal {
                source: "DPoP JWK is missing the private component (`d`)".into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        let x_b64 = jwk
            .get("x")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OAuthError::Internal {
                source: "DPoP JWK is missing `x`".into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        let y_b64 = jwk
            .get("y")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OAuthError::Internal {
                source: "DPoP JWK is missing `y`".into(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        let d = decode_b64url_bn(d_b64)?;
        let x = decode_b64url_bn(x_b64)?;
        let y = decode_b64url_bn(y_b64)?;

        let group =
            EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).context(DPoPProofGenerationSnafu)?;
        let public = openssl::ec::EcKey::from_public_key_affine_coordinates(&group, &x, &y)
            .context(DPoPProofGenerationSnafu)?;
        let key = EcKey::from_private_components(&group, &d, public.public_key())
            .context(DPoPProofGenerationSnafu)?;
        key.check_key().context(DPoPProofGenerationSnafu)?;
        Ok(Self { key })
    }

    /// Serialize the key as a JWK including the private component, so it
    /// can be reused across the token-acquisition leg and the Snowflake
    /// login-request leg (analysis §5.1 — the JDBC bundled-cache pattern).
    pub(crate) fn to_jwk_json(&self) -> Result<String, OAuthError> {
        let (x_b64, y_b64) = self.public_xy_b64()?;
        let d_b64 = bn_to_b64url(self.key.private_key())?;
        Ok(format!(
            r#"{{"crv":"P-256","d":"{d_b64}","kty":"EC","x":"{x_b64}","y":"{y_b64}"}}"#
        ))
    }

    fn public_xy_b64(&self) -> Result<(String, String), OAuthError> {
        let group = self.key.group();
        let mut ctx = BigNumContext::new().context(DPoPProofGenerationSnafu)?;
        let mut x = BigNum::new().context(DPoPProofGenerationSnafu)?;
        let mut y = BigNum::new().context(DPoPProofGenerationSnafu)?;
        self.key
            .public_key()
            .affine_coordinates_gfp(group, &mut x, &mut y, &mut ctx)
            .context(DPoPProofGenerationSnafu)?;
        Ok((bn_to_b64url(&x)?, bn_to_b64url(&y)?))
    }
}

fn bn_to_b64url(bn: &openssl::bn::BigNumRef) -> Result<String, OAuthError> {
    let bytes = bn
        .to_vec_padded(COORD_BYTES)
        .context(DPoPProofGenerationSnafu)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[allow(dead_code)]
fn decode_b64url_bn(s: &str) -> Result<BigNum, OAuthError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|e| OAuthError::Internal {
            source: Box::new(e),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;
    BigNum::from_slice(&bytes).context(DPoPProofGenerationSnafu)
}

/// Compute the canonical RFC 7638 JWK SHA-256 thumbprint for the public
/// portion of `key`. Output is base64url no-padding.
pub(crate) fn jwk_thumbprint(key: &DPoPKey) -> Result<String, OAuthError> {
    let (x, y) = key.public_xy_b64()?;
    let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(hasher.finish()))
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
        .map_err(|e| OAuthError::Internal {
            source: Box::new(e),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?
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
    let mut hasher = Sha256::new();
    hasher.update(signing_input.as_bytes());
    let digest = hasher.finish();

    let sig = EcdsaSig::sign(&digest, &key.key).context(DPoPProofGenerationSnafu)?;
    let r_bytes = sig
        .r()
        .to_vec_padded(COORD_BYTES)
        .context(DPoPProofGenerationSnafu)?;
    let s_bytes = sig
        .s()
        .to_vec_padded(COORD_BYTES)
        .context(DPoPProofGenerationSnafu)?;
    let mut jose = Vec::with_capacity(r_bytes.len() + s_bytes.len());
    jose.extend_from_slice(&r_bytes);
    jose.extend_from_slice(&s_bytes);
    let sig_b64 = URL_SAFE_NO_PAD.encode(jose);

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
/// with the nonce embedded in the proof (analysis §5.1; mirrors JDBC
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

    #[test]
    fn proof_jwt_signature_verifies_against_public_key() {
        let key = DPoPKey::generate().unwrap();
        let url = Url::parse("https://idp.example.com/oauth/token-request").unwrap();
        let jwt = proof_jwt(&key, "POST", &url, None).unwrap();
        let parts: Vec<&str> = jwt.reveal().split('.').collect();
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let mut hasher = Sha256::new();
        hasher.update(signing_input.as_bytes());
        let digest = hasher.finish();
        let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let (r_bytes, s_bytes) = sig_bytes.split_at(32);
        let r = BigNum::from_slice(r_bytes).unwrap();
        let s = BigNum::from_slice(s_bytes).unwrap();
        let ecdsa = EcdsaSig::from_private_components(r, s).unwrap();
        assert!(ecdsa.verify(&digest, &key.key).unwrap());
    }
}
