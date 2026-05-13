#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock as AsyncRwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::connection::RefreshContext;
use super::error::ApiError;
use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::{RestError, SessionTokens, heartbeat::send_heartbeat};

/// Absolute ceiling on the heartbeat interval, regardless of master validity.
const MAX_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3600);

/// Absolute floor on the heartbeat interval. Guards against sub-second busy
/// loops when `master_validity` is tiny (the window `[master/16, master/4]`
/// can collapse to zero for validities below 16 s).
const MIN_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

/// Divisors that, together with `master_validity`, define the safe clamp
/// window `[master/16, master/4]` — the window Python's connector uses.
const MIN_INTERVAL_DIVISOR: u32 = 16;
const MAX_INTERVAL_DIVISOR: u32 = 4;

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

/// Compute the heartbeat interval, honoring an optional user override and
/// Python's `[master_validity / 16, master_validity / 4]` clamp.
///
/// When `user_frequency` is set, it is clamped into the window so values the
/// server would consider too aggressive or too slack become the nearest safe
/// cadence. When it is not set, the default is `master_validity / 4`.
/// The result is always in `[MIN_HEARTBEAT_INTERVAL, MAX_HEARTBEAT_INTERVAL]`.
/// Falls back to `MAX_HEARTBEAT_INTERVAL` when master validity is unknown.
pub(crate) fn compute_heartbeat_interval(
    master_validity: Option<Duration>,
    user_frequency: Option<Duration>,
) -> Duration {
    let interval = match master_validity {
        None => user_frequency.unwrap_or(MAX_HEARTBEAT_INTERVAL),
        Some(master) => {
            let floor = master / MIN_INTERVAL_DIVISOR;
            let ceiling = master / MAX_INTERVAL_DIVISOR;
            user_frequency.unwrap_or(ceiling).clamp(floor, ceiling)
        }
    };
    interval.clamp(MIN_HEARTBEAT_INTERVAL, MAX_HEARTBEAT_INTERVAL)
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
) -> HeartbeatHandle {
    let cancel_token = CancellationToken::new();
    let task_token = cancel_token.clone();

    let task_handle = tokio::spawn(heartbeat_loop(
        tokens,
        http_client,
        server_url,
        client_info,
        heartbeat_interval,
        task_token,
    ));

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
        }
    }

    #[test]
    fn compute_interval_default() {
        // No master validity, no user override → absolute ceiling.
        let interval = compute_heartbeat_interval(None, None);
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn compute_interval_from_validity() {
        // 4h master → master/4 = 3600s, exactly the ceiling.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)), None);
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn compute_interval_no_bottom_clamp_without_master_override() {
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(100)), None);
        // master/4 with master=100s → 25s, below the absolute ceiling.
        assert_eq!(interval, Duration::from_secs(25));
    }

    #[test]
    fn compute_interval_clamp_max() {
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(100_000)), None);
        assert_eq!(interval, MAX_HEARTBEAT_INTERVAL);
    }

    #[test]
    fn user_frequency_within_window_passes_through() {
        // Master=14400s → window [900, 3600]. 1800s is within.
        let interval = compute_heartbeat_interval(
            Some(Duration::from_secs(14400)),
            Some(Duration::from_secs(1800)),
        );
        assert_eq!(interval, Duration::from_secs(1800));
    }

    #[test]
    fn user_frequency_below_floor_is_clamped_up() {
        // Master=14400s → floor=900s. 60s user hint clamps up.
        let interval = compute_heartbeat_interval(
            Some(Duration::from_secs(14400)),
            Some(Duration::from_secs(60)),
        );
        assert_eq!(interval, Duration::from_secs(900));
    }

    #[test]
    fn user_frequency_above_ceiling_is_clamped_down() {
        // Master=14400s → ceiling=3600s. 7200s user hint clamps down.
        let interval = compute_heartbeat_interval(
            Some(Duration::from_secs(14400)),
            Some(Duration::from_secs(7200)),
        );
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn tiny_master_validity_is_pinned_to_absolute_floor() {
        // With master=4s, master/16 and master/4 both round down to sub-second
        // Durations — the absolute floor prevents a busy loop.
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(4)), None);
        assert_eq!(interval, MIN_HEARTBEAT_INTERVAL);
    }

    #[test]
    fn user_frequency_without_master_still_capped_by_absolute_ceiling() {
        let interval = compute_heartbeat_interval(None, Some(Duration::from_secs(10_000)));
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
        ));

        // Wait for at least one heartbeat, then clear tokens.
        tokio::time::sleep(Duration::from_millis(100)).await;
        *tokens.write().await = None;

        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("task did not exit within timeout")
            .expect("task panicked");
    }
}
