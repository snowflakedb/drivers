//! DPoP proofs (RFC 9449) on requests that present a DPoP-bound token.

use reqwest::header::HeaderValue;

use crate::crypto::dpop::{DPoPKey, access_token_proof_jwt};
use crate::sensitive::SensitiveString;

const DPOP_HEADER: &str = "DPoP";

/// The key a token is bound to, and the token string a request presents.
#[derive(Clone, Debug)]
pub(crate) struct DPoPBinding {
    key: DPoPKey,
    access_token: SensitiveString,
}

impl DPoPBinding {
    pub(crate) fn new(key: &DPoPKey, access_token: &str) -> Self {
        Self {
            key: key.clone(),
            access_token: SensitiveString::from(access_token.to_owned()),
        }
    }

    /// Adds a proof for the request's method and URL, replacing any earlier one.
    ///
    /// A proof that cannot be built is left out. Snowflake rejects a bound session's request that
    /// carries no proof.
    pub(crate) fn sign(&self, request: &mut reqwest::Request) {
        let proof = match access_token_proof_jwt(
            &self.key,
            request.method().as_str(),
            request.url(),
            self.access_token.reveal(),
        ) {
            Ok(proof) => proof,
            Err(error) => {
                tracing::warn!(%error, "Failed to sign the DPoP proof; sending the request without one");
                return;
            }
        };
        match HeaderValue::from_str(proof.reveal()) {
            Ok(value) => {
                request.headers_mut().insert(DPOP_HEADER, value);
            }
            Err(error) => {
                tracing::warn!(%error, "DPoP proof is not a valid header value; sending the request without one");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::dpop::jwk_thumbprint;
    use crate::crypto::dpop::test_fixtures::{decode_jwt, header_jkt, token_hash};

    #[test]
    fn sign_adds_a_proof_for_the_request_and_token() {
        let key = DPoPKey::generate().unwrap();
        let mut request = reqwest::Client::new()
            .get("https://account.snowflakecomputing.com/monitoring/queries/q1?requestId=1")
            .build()
            .unwrap();

        DPoPBinding::new(&key, "session-token").sign(&mut request);

        let (header, claims) = decode_jwt(request.headers()[DPOP_HEADER].to_str().unwrap());
        assert_eq!(claims["htm"], "GET");
        assert_eq!(
            claims["htu"],
            "https://account.snowflakecomputing.com/monitoring/queries/q1"
        );
        assert_eq!(claims["ath"], token_hash("session-token"));
        assert_eq!(header_jkt(&header), jwk_thumbprint(&key).unwrap());
    }

    #[test]
    fn sign_replaces_an_earlier_proof() {
        let key = DPoPKey::generate().unwrap();
        let mut request = reqwest::Client::new()
            .post("https://account.snowflakecomputing.com/session/heartbeat")
            .header(DPOP_HEADER, "caller-supplied")
            .build()
            .unwrap();

        DPoPBinding::new(&key, "session-token").sign(&mut request);

        let proofs: Vec<_> = request.headers().get_all(DPOP_HEADER).iter().collect();
        assert_eq!(proofs.len(), 1);
        assert_ne!(proofs[0], "caller-supplied");
    }
}
