//! DPoP proofs (RFC 9449) on requests that present a DPoP-bound token.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use reqwest::header::{HeaderMap, HeaderValue, WWW_AUTHENTICATE};

use crate::crypto::dpop::{DPoPKey, access_token_proof_jwt};
use crate::sensitive::SensitiveString;
use crate::utils::sync::MutexRecoverExt;

const DPOP_HEADER: &str = "DPoP";
const DPOP_NONCE_HEADER: &str = "DPoP-Nonce";
/// The `error` of a `WWW-Authenticate: DPoP` challenge that wants a proof with the latest nonce
/// (RFC 9449 §9).
const USE_DPOP_NONCE: &str = "use_dpop_nonce";
/// Longest `DPoP-Nonce` put into proofs; a longer one is ignored.
const MAX_NONCE_LEN: usize = 1024;

/// The key a session is bound to, and the latest nonce Snowflake issued for it.
///
/// Clones share the nonce, so every request of the session proves with the newest one. Outside
/// the crate it is an opaque handle.
#[derive(Clone)]
pub struct DPoPSessionKey {
    key: DPoPKey,
    nonce: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for DPoPSessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DPoPSessionKey").finish_non_exhaustive()
    }
}

impl DPoPSessionKey {
    /// `nonce` is the one the login response issued, if any.
    pub(crate) fn new(key: DPoPKey, nonce: Option<String>) -> Self {
        Self {
            key,
            nonce: Arc::new(Mutex::new(nonce)),
        }
    }

    pub(crate) fn key(&self) -> &DPoPKey {
        &self.key
    }

    fn nonce(&self) -> Option<String> {
        self.nonce.lock_recover().clone()
    }

    /// Keeps the nonce a response issued, and returns it when it differs from the one `sent`.
    fn record_nonce(&self, headers: &HeaderMap, sent: Option<&str>) -> Option<String> {
        let issued = issued_nonce(headers)?;
        if sent == Some(issued.as_str()) {
            return None;
        }
        *self.nonce.lock_recover() = Some(issued.clone());
        Some(issued)
    }
}

/// The `DPoP-Nonce` a Snowflake response carries.
///
/// A nonce must be 1 to [`MAX_NONCE_LEN`] NQCHARs (RFC 9449 §8); anything else is ignored.
pub(crate) fn issued_nonce(headers: &HeaderMap) -> Option<String> {
    let nonce = headers.get(DPOP_NONCE_HEADER)?.as_bytes();
    let is_nqchar = |byte: &u8| matches!(byte, 0x21 | 0x23..=0x5B | 0x5D..=0x7E);
    if nonce.is_empty() || nonce.len() > MAX_NONCE_LEN || !nonce.iter().all(is_nqchar) {
        tracing::warn!(len = nonce.len(), "Ignoring a malformed DPoP-Nonce");
        return None;
    }
    String::from_utf8(nonce.to_vec()).ok()
}

/// The key a token is bound to, and the token string a request presents.
#[derive(Clone, Debug)]
pub(crate) struct DPoPBinding {
    session: DPoPSessionKey,
    access_token: SensitiveString,
}

impl DPoPBinding {
    pub(crate) fn new(session: &DPoPSessionKey, access_token: &str) -> Self {
        Self {
            session: session.clone(),
            access_token: SensitiveString::from(access_token.to_owned()),
        }
    }

    /// Sends `request` with a proof, and keeps any nonce the response issues.
    ///
    /// A response that issues a new nonce with a `WWW-Authenticate: DPoP error="use_dpop_nonce"`
    /// challenge gets the request sent once more, with a proof that carries the new nonce. A
    /// second challenge goes back to the caller.
    ///
    /// The resend ignores the method, so POSTs such as query submission are sent again too. That
    /// is safe only because Snowflake checks the proof before it runs a request: a challenged
    /// request was never executed.
    pub(crate) async fn send(
        &self,
        client: &reqwest::Client,
        mut request: reqwest::Request,
    ) -> reqwest::Result<reqwest::Response> {
        let retry = request.try_clone();
        let sent = self.session.nonce();
        self.sign(&mut request, sent.as_deref());
        let started = Instant::now();
        let response = client.execute(request).await?;
        let Some(issued) = self
            .session
            .record_nonce(response.headers(), sent.as_deref())
        else {
            return Ok(response);
        };

        if !is_nonce_challenge(response.headers()) {
            return Ok(response);
        }
        let Some(mut retry) = retry else {
            return Ok(response);
        };
        // The resend shares the first send's timeout rather than starting a new one.
        if let Some(timeout) = retry.timeout_mut() {
            *timeout = timeout.saturating_sub(started.elapsed());
            if timeout.is_zero() {
                return Ok(response);
            }
        }
        tracing::debug!(
            method = %retry.method(),
            host = retry.url().host_str().unwrap_or_default(),
            path = retry.url().path(),
            status = response.status().as_u16(),
            "Snowflake asked for a DPoP nonce; sending the request again with it"
        );
        drop(response);
        self.sign(&mut retry, Some(&issued));
        let response = client.execute(retry).await?;
        self.session.record_nonce(response.headers(), Some(&issued));
        Ok(response)
    }

    /// Adds a proof for the request's method and URL, replacing any earlier one.
    ///
    /// A proof that cannot be built is left out. Snowflake rejects a bound session's request that
    /// carries no proof.
    fn sign(&self, request: &mut reqwest::Request, nonce: Option<&str>) {
        let proof = match access_token_proof_jwt(
            self.session.key(),
            request.method().as_str(),
            request.url(),
            nonce,
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

/// Sends `request`, through `dpop` when the session is bound.
pub(crate) async fn execute(
    client: &reqwest::Client,
    request: reqwest::Request,
    dpop: Option<DPoPBinding>,
) -> reqwest::Result<reqwest::Response> {
    match dpop {
        Some(dpop) => dpop.send(client, request).await,
        None => client.execute(request).await,
    }
}

/// Whether a response challenges the proof's nonce (RFC 9449 §9).
///
/// Snowflake sends the challenge on driver endpoints with HTTP 200 rather than 401, which the
/// driver treats as an expired session.
fn is_nonce_challenge(headers: &HeaderMap) -> bool {
    headers
        .get_all(WWW_AUTHENTICATE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| {
            let (scheme, params) = value.trim().split_once(' ')?;
            scheme.eq_ignore_ascii_case("DPoP").then_some(params)
        })
        .flat_map(|params| params.split(','))
        .filter_map(|param| param.split_once('='))
        .any(|(name, value)| {
            name.trim().eq_ignore_ascii_case("error")
                && value.trim().trim_matches('"') == USE_DPOP_NONCE
        })
}

#[cfg(test)]
pub(crate) mod test_fixtures {
    use super::DPoPSessionKey;
    use crate::crypto::dpop::DPoPKey;

    /// A fresh session key with no nonce yet.
    pub(crate) fn session_key() -> DPoPSessionKey {
        DPoPSessionKey::new(DPoPKey::generate().unwrap(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixtures::session_key;
    use super::*;
    use crate::crypto::dpop::jwk_thumbprint;
    use crate::crypto::dpop::test_fixtures::{decode_jwt, header_jkt, token_hash};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn challenge() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .insert_header(DPOP_NONCE_HEADER, "nonce-1")
            .insert_header(
                WWW_AUTHENTICATE,
                r#"DPoP error="use_dpop_nonce", algs="ES256""#,
            )
            .set_body_json(serde_json::json!({
                "success": false,
                "message": "DPoP nonce required",
                "data": { "dpopError": USE_DPOP_NONCE },
            }))
    }

    fn heartbeat(server: &MockServer) -> reqwest::Request {
        reqwest::Client::new()
            .post(format!("{}/session/heartbeat", server.uri()))
            .body("{}")
            .build()
            .unwrap()
    }

    async fn proof_claims(server: &MockServer) -> Vec<serde_json::Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| decode_jwt(request.headers[DPOP_HEADER].to_str().unwrap()).1)
            .collect()
    }

    #[test]
    fn sign_adds_a_proof_for_the_request_and_token() {
        let session = session_key();
        let mut request = reqwest::Client::new()
            .get("https://account.snowflakecomputing.com/monitoring/queries/q1?requestId=1")
            .build()
            .unwrap();

        DPoPBinding::new(&session, "session-token").sign(&mut request, None);

        let (header, claims) = decode_jwt(request.headers()[DPOP_HEADER].to_str().unwrap());
        assert_eq!(claims["htm"], "GET");
        assert_eq!(
            claims["htu"],
            "https://account.snowflakecomputing.com/monitoring/queries/q1"
        );
        assert_eq!(claims["ath"], token_hash("session-token"));
        assert!(claims.get("nonce").is_none());
        assert_eq!(header_jkt(&header), jwk_thumbprint(session.key()).unwrap());
    }

    #[test]
    fn sign_replaces_an_earlier_proof() {
        let mut request = reqwest::Client::new()
            .post("https://account.snowflakecomputing.com/session/heartbeat")
            .header(DPOP_HEADER, "caller-supplied")
            .build()
            .unwrap();

        DPoPBinding::new(&session_key(), "session-token").sign(&mut request, None);

        let proofs: Vec<_> = request.headers().get_all(DPOP_HEADER).iter().collect();
        assert_eq!(proofs.len(), 1);
        assert_ne!(proofs[0], "caller-supplied");
    }

    #[tokio::test]
    async fn later_proofs_carry_the_nonce_a_response_issued() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).insert_header(DPOP_NONCE_HEADER, "nonce-1"))
            .mount(&server)
            .await;
        let session = session_key();
        let client = reqwest::Client::new();

        for _ in 0..2 {
            DPoPBinding::new(&session, "session-token")
                .send(&client, heartbeat(&server))
                .await
                .unwrap();
        }

        let claims = proof_claims(&server).await;
        assert!(claims[0].get("nonce").is_none());
        assert_eq!(claims[1]["nonce"], "nonce-1");
    }

    #[tokio::test]
    async fn a_nonce_challenge_is_answered_once_with_the_new_nonce() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(challenge())
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
            )
            .mount(&server)
            .await;

        let response = DPoPBinding::new(&session_key(), "session-token")
            .send(&reqwest::Client::new(), heartbeat(&server))
            .await
            .unwrap();

        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["success"],
            true
        );
        let claims = proof_claims(&server).await;
        assert_eq!(claims.len(), 2);
        assert!(claims[0].get("nonce").is_none());
        assert_eq!(claims[1]["nonce"], "nonce-1");
        assert_ne!(claims[0]["jti"], claims[1]["jti"]);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests[1].body, b"{}");
    }

    #[tokio::test]
    async fn a_second_challenge_goes_back_to_the_caller() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(challenge())
            .mount(&server)
            .await;

        let response = DPoPBinding::new(&session_key(), "session-token")
            .send(&reqwest::Client::new(), heartbeat(&server))
            .await
            .unwrap();

        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["data"]["dpopError"],
            USE_DPOP_NONCE
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_body_naming_the_error_without_the_challenge_header_is_not_sent_again() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(DPOP_NONCE_HEADER, "nonce-1")
                    .set_body_json(serde_json::json!({
                        "success": false,
                        "data": { "dpopError": USE_DPOP_NONCE },
                    })),
            )
            .mount(&server)
            .await;

        DPoPBinding::new(&session_key(), "session-token")
            .send(&reqwest::Client::new(), heartbeat(&server))
            .await
            .unwrap();

        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[test]
    fn the_challenge_is_recognized_with_or_without_quotes() {
        for value in [
            r#"DPoP error="use_dpop_nonce", algs="ES256""#,
            "DPoP error=use_dpop_nonce",
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(WWW_AUTHENTICATE, HeaderValue::from_str(value).unwrap());
            assert!(is_nonce_challenge(&headers), "{value}");
        }
        for value in [
            r#"DPoP error="invalid_dpop_proof""#,
            "DPoP error=use_dpop_nonce_foo",
            r#"Bearer error="use_dpop_nonce""#,
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(WWW_AUTHENTICATE, HeaderValue::from_str(value).unwrap());
            assert!(!is_nonce_challenge(&headers), "{value}");
        }
    }

    #[test]
    fn a_malformed_nonce_is_ignored() {
        let oversized = "n".repeat(MAX_NONCE_LEN + 1);
        for value in ["", oversized.as_str(), "nonce 1", r#"nonce"1"#, r"nonce\1"] {
            let mut headers = HeaderMap::new();
            headers.insert(DPOP_NONCE_HEADER, HeaderValue::from_str(value).unwrap());
            assert!(issued_nonce(&headers).is_none(), "{value:?}");
        }

        let mut headers = HeaderMap::new();
        headers.insert(DPOP_NONCE_HEADER, HeaderValue::from_static("nonce-1"));
        assert_eq!(issued_nonce(&headers).as_deref(), Some("nonce-1"));
    }

    #[tokio::test]
    async fn a_challenge_repeating_the_nonce_just_sent_is_not_sent_again() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(challenge())
            .mount(&server)
            .await;
        let session = DPoPSessionKey::new(DPoPKey::generate().unwrap(), Some("nonce-1".into()));

        DPoPBinding::new(&session, "session-token")
            .send(&reqwest::Client::new(), heartbeat(&server))
            .await
            .unwrap();

        let claims = proof_claims(&server).await;
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0]["nonce"], "nonce-1");
    }

    #[tokio::test]
    async fn a_challenge_without_a_nonce_is_not_sent_again() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(WWW_AUTHENTICATE, r#"DPoP error="use_dpop_nonce""#),
            )
            .mount(&server)
            .await;

        DPoPBinding::new(&session_key(), "session-token")
            .send(&reqwest::Client::new(), heartbeat(&server))
            .await
            .unwrap();

        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
