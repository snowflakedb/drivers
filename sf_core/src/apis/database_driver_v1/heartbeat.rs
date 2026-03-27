use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock as AsyncRwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::{
    self, RestError, SessionTokens, SnowflakeResponseError, heartbeat::send_heartbeat,
};

const MIN_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(900);
const MAX_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3600);

/// Handle to a per-connection heartbeat background task.
///
/// Cancels the task on drop to ensure cleanup when the connection is released.
pub(crate) struct HeartbeatHandle {
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

/// Compute the heartbeat interval from master token validity.
///
/// Returns `master_validity / 4`, clamped to `[900s, 3600s]`.
/// Falls back to 3600s if `master_validity` is `None`.
pub(crate) fn compute_heartbeat_interval(master_validity: Option<Duration>) -> Duration {
    master_validity
        .map(|v| (v / 4).clamp(MIN_HEARTBEAT_INTERVAL, MAX_HEARTBEAT_INTERVAL))
        .unwrap_or(MAX_HEARTBEAT_INTERVAL)
}

/// Spawn a per-connection heartbeat background task.
///
/// The task sends periodic `POST /session/heartbeat` requests to keep the
/// session alive. It automatically refreshes the session token on 401
/// responses, and exits when cancelled or when the session tokens are cleared.
pub(crate) fn spawn_heartbeat_task(
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

    loop {
        tokio::select! {
            () = tokio::time::sleep(interval) => {}
            () = cancel_token.cancelled() => {
                tracing::info!("Heartbeat task cancelled");
                return;
            }
        }

        if cancel_token.is_cancelled() {
            tracing::info!("Heartbeat task cancelled after wakeup");
            return;
        }

        let session_token = {
            let guard = tokens.read().await;
            match guard.as_ref() {
                Some(t) => t.session_token.clone(),
                None => {
                    tracing::info!("Session tokens cleared, heartbeat task exiting");
                    return;
                }
            }
        };

        let result = send_heartbeat(
            &http_client,
            &server_url,
            &client_info,
            session_token.reveal(),
        )
        .await;

        match result {
            Ok(()) => {
                tracing::debug!("Heartbeat succeeded");
            }
            Err(RestError::InvalidSnowflakeResponse {
                source: SnowflakeResponseError::SessionExpired { .. },
                ..
            }) => {
                tracing::info!("Heartbeat got 401, attempting token refresh");
                if !try_refresh_and_retry(
                    &tokens,
                    &http_client,
                    &server_url,
                    &client_info,
                    &session_token,
                )
                .await
                {
                    return;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Heartbeat failed, will retry next interval");
            }
        }
    }
}

/// Attempt to refresh the session token and retry the heartbeat once.
///
/// Returns `true` if the loop should continue, `false` if the task should exit.
async fn try_refresh_and_retry(
    tokens: &Arc<AsyncRwLock<Option<SessionTokens>>>,
    http_client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    failed_token: &crate::sensitive::SensitiveString,
) -> bool {
    let mut guard = tokens.write().await;

    let current_tokens = match guard.as_ref() {
        Some(t) => t.clone(),
        None => {
            tracing::info!("Session tokens cleared during refresh, heartbeat task exiting");
            return false;
        }
    };

    // Another task may have already refreshed the token while we waited for the write lock.
    if current_tokens.session_token.reveal() != failed_token.reveal() {
        tracing::debug!("Session already refreshed by another task, retrying heartbeat");
        let new_token = current_tokens.session_token.clone();
        drop(guard);
        return retry_heartbeat(http_client, server_url, client_info, &new_token).await;
    }

    if current_tokens.is_master_expired() {
        tracing::error!("Master token expired, heartbeat task exiting");
        return false;
    }

    match snowflake::refresh_session(http_client, server_url, client_info, &current_tokens).await {
        Ok(new_tokens) => {
            let new_token = new_tokens.session_token.clone();
            *guard = Some(new_tokens);
            drop(guard);
            tracing::info!("Session refreshed by heartbeat task");
            retry_heartbeat(http_client, server_url, client_info, &new_token).await
        }
        Err(e) => {
            tracing::error!(error = %e, "Heartbeat token refresh failed, task exiting");
            false
        }
    }
}

async fn retry_heartbeat(
    http_client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    token: &crate::sensitive::SensitiveString,
) -> bool {
    match send_heartbeat(http_client, server_url, client_info, token.reveal()).await {
        Ok(()) => {
            tracing::debug!("Heartbeat retry succeeded after refresh");
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, "Heartbeat retry failed after refresh");
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::crl::config::CrlConfig;
    use crate::sensitive::SensitiveString;
    use crate::tls::config::TlsConfig;

    fn test_client_info() -> ClientInfo {
        ClientInfo {
            application: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            os: "Linux".to_string(),
            os_version: "5.15".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::default(),
        }
    }

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
        let interval = compute_heartbeat_interval(None);
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn compute_interval_from_validity() {
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(14400)));
        assert_eq!(interval, Duration::from_secs(3600));
    }

    #[test]
    fn compute_interval_clamp_min() {
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(100)));
        assert_eq!(interval, MIN_HEARTBEAT_INTERVAL);
    }

    #[test]
    fn compute_interval_clamp_max() {
        let interval = compute_heartbeat_interval(Some(Duration::from_secs(100_000)));
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
