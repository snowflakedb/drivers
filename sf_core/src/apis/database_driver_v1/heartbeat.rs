#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use tokio::sync::RwLock as AsyncRwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::instrument::WithSubscriber;
use url::Url;

use super::connection::RefreshContext;
use super::error::ApiError;
use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::{RestError, SessionTokens, heartbeat::send_heartbeat};

const MAX_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3600);

/// Handle to a per-connection heartbeat background task.
///
/// Cancels the task on drop to ensure cleanup when the connection is released.
pub struct HeartbeatHandle {
    cancel_token: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
}

impl HeartbeatHandle {
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Cancel the task and wait for it to finish.
    pub async fn cancel_and_wait(&mut self) {
        self.cancel_token.cancel();
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for HeartbeatHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

/// Compute the heartbeat interval from master token validity and an optional
/// user-requested frequency (in seconds).
///
/// When `user_frequency_secs` is `None`, returns `master_validity / 16` —
/// matching the Python connector's default
/// (`_validate_client_session_keep_alive_heartbeat_frequency`).
///
/// When `user_frequency_secs` is provided, it is clamped to the closed range
/// `[master_validity / 16, master_validity / 4]`.
///
/// In both cases the final value is capped at [`MAX_HEARTBEAT_INTERVAL`].
/// When `master_validity` is `None` the master validity defaults to 4h
/// (the Snowflake server default), giving `[900s, 3600s]`.
pub fn compute_heartbeat_interval(
    master_validity: Option<Duration>,
    user_frequency_secs: Option<u64>,
) -> Duration {
    const DEFAULT_MASTER_VALIDITY: Duration = Duration::from_secs(4 * 3600);
    let master = master_validity.unwrap_or(DEFAULT_MASTER_VALIDITY);
    let max = master / 4;
    let min = master / 16;

    user_frequency_secs
        .map_or(min, |s| Duration::from_secs(s).clamp(min, max))
        .min(MAX_HEARTBEAT_INTERVAL)
}

/// Spawn a per-connection heartbeat background task.
///
/// The task sends periodic `POST /session/heartbeat` requests to keep the
/// session alive. It automatically refreshes the session token on 401
/// responses, and exits when cancelled or when the session tokens are cleared.
pub fn spawn_heartbeat_task(
    tokens: Arc<AsyncRwLock<Option<SessionTokens>>>,
    http_client: reqwest::Client,
    server_url: String,
    client_info: ClientInfo,
    heartbeat_interval: Duration,
    is_master_token_expired: Arc<AtomicBool>,
) -> HeartbeatHandle {
    let cancel_token = CancellationToken::new();
    let task_token = cancel_token.clone();

    let task_handle = tokio::spawn(
        heartbeat_loop(
            tokens,
            http_client,
            server_url,
            client_info,
            heartbeat_interval,
            task_token,
            is_master_token_expired,
        )
        .with_current_subscriber(),
    );

    HeartbeatHandle {
        cancel_token,
        task_handle: Some(task_handle),
    }
}

async fn heartbeat_loop(
    tokens: Arc<AsyncRwLock<Option<SessionTokens>>>,
    http_client: reqwest::Client,
    server_url: String,
    client_info: ClientInfo,
    interval: Duration,
    cancel_token: CancellationToken,
    is_master_token_expired: Arc<AtomicBool>,
) {
    tracing::info!(interval_secs = interval.as_secs(), "Heartbeat task started");

    let server_url = match Url::parse(&server_url) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, "Invalid server URL, heartbeat task exiting");
            return;
        }
    };

    loop {
        tokio::select! {
            () = tokio::time::sleep(interval) => {}
            () = cancel_token.cancelled() => {
                tracing::info!("Heartbeat task cancelled");
                return;
            }
        }

        let mut ctx = RefreshContext::from_parts(
            tokens.clone(),
            http_client.clone(),
            server_url.to_string(),
            client_info.clone(),
            is_master_token_expired.clone(),
        );
        let mut last_error: Option<RestError> = None;
        loop {
            let token = tokio::select! {
                result = ctx.refresh_token(last_error.take()) => match result {
                    Ok(t) => t,
                    Err(ApiError::MasterTokenExpired { .. }) => {
                        tracing::error!("Master token expired, heartbeat task exiting");
                        return;
                    }
                    Err(ApiError::ConnectionNotInitialized { .. }) => {
                        tracing::info!("Session tokens cleared, heartbeat task exiting");
                        return;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Heartbeat failed, will retry next interval");
                        break;
                    }
                },
                () = cancel_token.cancelled() => {
                    tracing::info!("Heartbeat task cancelled during token refresh");
                    return;
                }
            };

            let result = tokio::select! {
                res = send_heartbeat(&http_client, &server_url, &client_info, token.reveal()) => res,
                () = cancel_token.cancelled() => {
                    tracing::info!("Heartbeat task cancelled during heartbeat request");
                    return;
                }
            };

            match result {
                Ok(()) => {
                    tracing::debug!("Heartbeat succeeded");
                    break;
                }
                Err(e) => last_error = Some(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::config::rest_parameters::test_fixtures::test_client_info;
    use crate::sensitive::SensitiveString;

    fn test_tokens(session_token: &str) -> SessionTokens {
        SessionTokens {
            session_token: SensitiveString::from(session_token.to_string()),
            master_token: SensitiveString::from("master_token".to_string()),
            session_id: 1,
            session_expires_at: None,
            master_expires_at: Some(std::time::Instant::now() + Duration::from_secs(14400)),
            master_validity: Some(Duration::from_secs(14400)),
        }
    }

    #[test]
    fn compute_interval_default_uses_master_over_16() {
        // No master validity, no user frequency -> default_master / 16 = 14400 / 16 = 900s.
        let interval = compute_heartbeat_interval(None, None);
        assert_eq!(interval, Duration::from_secs(900));
    }

    #[test]
    fn compute_interval_default_from_validity() {
        // validity / 16
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)), None);
        assert_eq!(interval, Duration::from_secs(900));
    }

    #[test]
    fn compute_interval_user_value_within_range() {
        // 1800 is within [900, 3600] for master=14400.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)), Some(1800));
        assert_eq!(interval, Duration::from_secs(1800));
    }

    #[test]
    fn compute_interval_user_value_clamped_to_min() {
        // 100 is below 900; clamp up.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)), Some(100));
        assert_eq!(interval, Duration::from_secs(900));
    }

    #[test]
    fn compute_interval_user_value_clamped_to_max() {
        // 9000 is above 3600; clamp down.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)), Some(9000));
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn compute_interval_clamp_max_overall() {
        // With a master of 100_000s, default would be 6250s; cap at MAX_HEARTBEAT_INTERVAL.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(100_000)), None);
        assert_eq!(interval, MAX_HEARTBEAT_INTERVAL);
    }

    #[tokio::test]
    async fn cancel_token_stops_task() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&server)
            .await;

        let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok"))));
        let mut handle = spawn_heartbeat_task(
            tokens,
            reqwest::Client::new(),
            server.uri(),
            test_client_info(),
            Duration::from_secs(3600),
            Arc::new(AtomicBool::new(false)),
        );

        handle.cancel_and_wait().await;
    }

    #[tokio::test]
    async fn task_exits_when_tokens_none() {
        let tokens: Arc<AsyncRwLock<Option<SessionTokens>>> = Arc::new(AsyncRwLock::new(None));

        let cancel_token = CancellationToken::new();
        let task_token = cancel_token.clone();

        let task = tokio::spawn(heartbeat_loop(
            tokens,
            reqwest::Client::new(),
            "http://localhost:1".to_string(),
            test_client_info(),
            Duration::from_millis(10),
            task_token,
            Arc::new(AtomicBool::new(false)),
        ));

        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("task did not exit within timeout")
            .expect("task panicked");
    }

    #[tokio::test]
    async fn task_sends_heartbeat_and_continues() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .expect(2..)
            .mount(&server)
            .await;

        let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok"))));
        let mut handle = spawn_heartbeat_task(
            tokens,
            reqwest::Client::new(),
            server.uri(),
            test_client_info(),
            Duration::from_millis(50),
            Arc::new(AtomicBool::new(false)),
        );

        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.cancel_and_wait().await;
    }

    #[tokio::test]
    async fn task_exits_when_tokens_cleared_mid_loop() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&server)
            .await;

        let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok"))));
        let tokens_clone = tokens.clone();

        let cancel_token = CancellationToken::new();
        let task_token = cancel_token.clone();

        let task = tokio::spawn(heartbeat_loop(
            tokens_clone,
            reqwest::Client::new(),
            server.uri(),
            test_client_info(),
            Duration::from_millis(50),
            task_token,
            Arc::new(AtomicBool::new(false)),
        ));

        // Wait for at least one heartbeat, then clear tokens.
        tokio::time::sleep(Duration::from_millis(100)).await;
        *tokens.write().await = None;

        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("task did not exit within timeout")
            .expect("task panicked");
    }

    /// A 390114 (master token expired) returned while the heartbeat task refreshes
    /// must set the shared `is_master_token_expired` flag, so the owning connection
    /// reports `is_expired == true` even though the expiry was detected on the
    /// background heartbeat path rather than a foreground query.
    #[tokio::test]
    async fn heartbeat_390114_sets_shared_expired_flag() {
        let server = MockServer::start().await;

        // Heartbeat gets 401 → triggers a session-token refresh.
        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        // The refresh endpoint reports the master token has expired (390114).
        Mock::given(method("POST"))
            .and(path("/session/token-request"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "code": "390114",
                "message": "Master token has expired. The session is no longer active."
            })))
            .mount(&server)
            .await;

        let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok"))));
        let is_master_token_expired = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_heartbeat_task(
            tokens,
            reqwest::Client::new(),
            server.uri(),
            test_client_info(),
            Duration::from_millis(20),
            is_master_token_expired.clone(),
        );

        // Poll for the flag rather than sleeping a fixed duration.
        let flagged = tokio::time::timeout(Duration::from_secs(2), async {
            while !is_master_token_expired.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok();
        handle.cancel_and_wait().await;

        assert!(
            flagged,
            "heartbeat-path 390114 must set the shared is_master_token_expired flag"
        );
    }
}
