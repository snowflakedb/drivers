//! Shared fixtures for the telemetry integration tests.
//!
//! One harness for both telemetry lanes (span exporter and raw log batch) so
//! their tests can't drift from each other or from the wire contract.

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, RwLock};

use serde_json::Value;
use sf_core::config::rest_parameters::QueryParameters;
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::rest::snowflake::SessionTokens;
use sf_core::sensitive::SensitiveString;
use sf_core::telemetry::snowflake_exporter::{ExporterSession, SessionRegistry};
use tokio::sync::RwLock as AsyncRwLock;

/// Session id used by the fixtures below; also the value stamped into
/// [`make_active_session`]'s token.
pub const SESSION_ID: i64 = 42;

pub fn test_query_parameters(server_url: &str) -> QueryParameters {
    QueryParameters {
        server_url: server_url.to_string(),
        client_info: test_client_info(),
        log_max_query_length: 80,
        log_query_text: false,
        log_query_parameters: false,
    }
}

/// Decompress a gzip-encoded request body and parse it as JSON.
pub fn decompress_gzip_json(body: &[u8]) -> Value {
    let mut decoder = flate2::read::GzDecoder::new(body);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("Failed to decompress gzip body");
    serde_json::from_str(&decompressed).expect("Failed to parse decompressed JSON")
}

/// Build an [`ExporterSession`] pointing at `server_url` with the given token.
pub fn make_session(server_url: &str, token: Option<SessionTokens>) -> Arc<ExporterSession> {
    Arc::new(ExporterSession {
        client: reqwest::Client::new(),
        query_parameters: test_query_parameters(server_url),
        session_token: Arc::new(AsyncRwLock::new(token)),
    })
}

/// Build an [`ExporterSession`] with a live session token.
pub fn make_active_session(server_url: &str) -> Arc<ExporterSession> {
    let tokens = SessionTokens {
        session_token: SensitiveString::from("test_token"),
        master_token: SensitiveString::from("master_token"),
        session_id: SESSION_ID,
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
    };
    make_session(server_url, Some(tokens))
}

/// A registry holding a single `session_id -> session` mapping.
pub fn make_registry(session_id: i64, session: Arc<ExporterSession>) -> SessionRegistry {
    let mut map = HashMap::new();
    map.insert(session_id, session);
    Arc::new(RwLock::new(map))
}

/// An empty registry — models telemetry being disabled for every session.
pub fn empty_registry() -> SessionRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}
