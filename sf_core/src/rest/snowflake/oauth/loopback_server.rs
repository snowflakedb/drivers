//! Loopback HTTP server that receives the IdP's authorization-code redirect.
//!
//! Must bind explicitly to `127.0.0.1` (and only `::1` when the user
//! supplies a literal IPv6 redirect URI). Do **not** replicate Node's
//! `0.0.0.0` bind — it widens the OAuth-window attack surface to the
//! local network.
//!
//! HTTP parsing and response writing are delegated to `axum`; we keep
//! ownership of the bind decision and the single-shot semantics
//! (oneshot channel + graceful shutdown).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use serde::Deserialize;
use snafu::ResultExt;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use url::Url;

use super::error::{
    BrowserTimeoutSnafu, IdpSnafu, MissingAuthorizationCodeSnafu, OAuthError, PortBindSnafu,
    RedirectUriParseSnafu,
};
use crate::sensitive::SensitiveString;

const SUCCESS_HTML: &str = concat!(
    "<!doctype html><html><head><meta charset=\"utf-8\">",
    "<title>Authorization Complete</title></head>",
    "<body><h1>Authorization completed successfully.</h1>",
    "<p>You may close this window.</p></body></html>"
);

const ERROR_HTML: &str = concat!(
    "<!doctype html><html><body>",
    "<h1>Authorization failed.</h1>",
    "<p>You may close this window and check the driver logs.</p>",
    "</body></html>"
);

/// Bound loopback listener + canonical redirect URI to advertise to the IdP.
pub(crate) struct LoopbackBinding {
    listener: TcpListener,
    pub(crate) redirect_uri: Url,
}

/// Parsed redirect from the browser.
#[derive(Debug)]
pub(crate) struct RedirectResult {
    pub(crate) code: SensitiveString,
    pub(crate) state: String,
}

/// Query string shape we expect on the OAuth callback. Axum's `Query`
/// extractor does the percent-decoding.
#[derive(Debug, Deserialize)]
struct RedirectQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Bind a loopback HTTP listener for the OAuth callback.
///
/// * If `redirect_uri_hint` is `Some`, the listener attempts to bind on
///   that host/port; the path is preserved (defaulting to `/`).
/// * Otherwise binds `127.0.0.1:0` (ephemeral) and surfaces the assigned
///   port back through `redirect_uri`.
///
/// We only ever bind to a loopback interface (gotcha §14 #11): IPv4
/// `127.0.0.1` by default, IPv6 `::1` only if the caller explicitly
/// supplied an IPv6 literal in the hint.
pub(crate) async fn bind(redirect_uri_hint: Option<&Url>) -> Result<LoopbackBinding, OAuthError> {
    let (ip, port, path) = match redirect_uri_hint {
        Some(url) => {
            let host = url.host();
            let ip = match host {
                Some(url::Host::Ipv4(v4)) => IpAddr::V4(v4),
                Some(url::Host::Ipv6(v6)) => IpAddr::V6(v6),
                _ => IpAddr::V4(Ipv4Addr::LOCALHOST),
            };
            let port = url.port().unwrap_or(0);
            let mut path = url.path().to_string();
            if path.is_empty() {
                path = "/".to_string();
            }
            (ip, port, path)
        }
        None => (IpAddr::V4(Ipv4Addr::LOCALHOST), 0, "/".to_string()),
    };

    let listener = TcpListener::bind(SocketAddr::new(ip, port))
        .await
        .context(PortBindSnafu)?;
    let local = listener.local_addr().context(PortBindSnafu)?;

    let host_for_url = match local.ip() {
        IpAddr::V4(_) => "127.0.0.1".to_string(),
        IpAddr::V6(_) => "[::1]".to_string(),
    };
    let redirect_uri = Url::parse(&format!("http://{host_for_url}:{}{path}", local.port()))
        .context(RedirectUriParseSnafu)?;

    tracing::debug!(
        bound = %local,
        redirect_uri_authority = %redirect_uri.authority(),
        redirect_uri_path = %redirect_uri.path(),
        "OAuth loopback listener bound"
    );

    Ok(LoopbackBinding {
        listener,
        redirect_uri,
    })
}

impl LoopbackBinding {
    /// Wait for the browser's redirect, parse `code`+`state` (or `error`)
    /// from the request's query string, respond with a small HTML page,
    /// then return.
    ///
    /// `timeout` is the total deadline for receiving the redirect,
    /// mirroring the browser-response-timeout knob in JDBC and ODBC.
    pub(crate) async fn wait_for_redirect(
        self,
        timeout: Duration,
    ) -> Result<RedirectResult, OAuthError> {
        let LoopbackBinding {
            listener,
            redirect_uri,
        } = self;

        let (result_tx, result_rx) = oneshot::channel::<Result<RedirectResult, OAuthError>>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let state = RedirectState {
            tx: Arc::new(Mutex::new(Some(result_tx))),
            shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
        };

        let app: Router<()> = Router::new()
            .route(redirect_uri.path(), get(handle_redirect))
            .with_state(state);

        let server_task = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        let (outcome, handler_fired) = tokio::select! {
            received = result_rx => {
                (received.unwrap_or_else(|_| BrowserTimeoutSnafu.fail()), true)
            }
            _ = tokio::time::sleep(timeout) => {
                (BrowserTimeoutSnafu.fail(), false)
            }
        };

        if handler_fired {
            // Handler triggered the graceful shutdown; give axum a brief
            // window to drain the response back to the browser.
            let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
        } else {
            // Timeout path — abort the spawn so the listener is released.
            server_task.abort();
            let _ = server_task.await;
        }

        outcome
    }
}

type RedirectSender = oneshot::Sender<Result<RedirectResult, OAuthError>>;

#[derive(Clone)]
struct RedirectState {
    tx: Arc<Mutex<Option<RedirectSender>>>,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[tracing::instrument(skip(state, query))]
async fn handle_redirect(
    State(state): State<RedirectState>,
    Query(query): Query<RedirectQuery>,
) -> Html<&'static str> {
    tracing::debug!("OAuth loopback handler received redirect");
    let result = parse_redirect_query(query);
    let body = if result.is_ok() {
        SUCCESS_HTML
    } else {
        ERROR_HTML
    };

    if let Some(tx) = state.tx.lock().await.take() {
        let _ = tx.send(result);
    }
    if let Some(stop) = state.shutdown.lock().await.take() {
        let _ = stop.send(());
    }

    Html(body)
}

fn parse_redirect_query(query: RedirectQuery) -> Result<RedirectResult, OAuthError> {
    if let Some(error) = query.error {
        return IdpSnafu {
            error,
            description: query.error_description.unwrap_or_default(),
        }
        .fail();
    }
    let Some(code) = query.code.filter(|s| !s.is_empty()) else {
        return MissingAuthorizationCodeSnafu.fail();
    };
    Ok(RedirectResult {
        code: SensitiveString::from(code),
        state: query.state.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn bind_uses_loopback_ipv4_by_default() {
        let b = bind(None).await.expect("bind");
        let addr = b.listener.local_addr().unwrap();
        match addr.ip() {
            IpAddr::V4(v4) => assert!(v4.is_loopback(), "non-loopback ipv4 bind: {v4}"),
            IpAddr::V6(_) => panic!("default bind should be ipv4 loopback"),
        }
        assert_eq!(b.redirect_uri.scheme(), "http");
        assert_eq!(b.redirect_uri.host_str(), Some("127.0.0.1"));
        assert!(b.redirect_uri.port().is_some());
    }

    #[tokio::test]
    async fn bind_honors_hint_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let hint = Url::parse(&format!("http://127.0.0.1:{port}/cb")).unwrap();
        let b = bind(Some(&hint)).await.expect("bind on hint port");
        assert_eq!(b.redirect_uri.port(), Some(port));
        assert_eq!(b.redirect_uri.path(), "/cb");
    }

    #[tokio::test]
    async fn parses_code_and_state_from_redirect() {
        let b = bind(None).await.expect("bind");
        let addr = b.listener.local_addr().unwrap();

        let join = tokio::spawn(async move { b.wait_for_redirect(Duration::from_secs(5)).await });
        let mut s = TcpStream::connect(addr).await.expect("connect loopback");
        s.write_all(
            b"GET /?code=AUTH-CODE-123&state=STATE-XYZ HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        )
        .await
        .unwrap();
        let res = join.await.unwrap().expect("wait_for_redirect");
        assert_eq!(res.code.reveal(), "AUTH-CODE-123");
        assert_eq!(res.state, "STATE-XYZ");
    }

    #[tokio::test]
    async fn parses_idp_error_from_redirect() {
        let b = bind(None).await.expect("bind");
        let addr = b.listener.local_addr().unwrap();

        let join = tokio::spawn(async move { b.wait_for_redirect(Duration::from_secs(5)).await });
        let mut s = TcpStream::connect(addr).await.expect("connect loopback");
        s.write_all(
            b"GET /?error=access_denied&error_description=user%20declined HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        )
        .await
        .unwrap();
        let err = join.await.unwrap().expect_err("error redirect must fail");
        match err {
            OAuthError::IdpError {
                error, description, ..
            } => {
                assert_eq!(error, "access_denied");
                assert_eq!(description, "user declined");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_surfaces_browser_timeout() {
        let b = bind(None).await.expect("bind");
        let err = b
            .wait_for_redirect(Duration::from_millis(50))
            .await
            .expect_err("must time out");
        assert!(matches!(err, OAuthError::BrowserTimeout { .. }));
    }

    #[tokio::test]
    async fn missing_code_surfaces_specific_error() {
        let b = bind(None).await.expect("bind");
        let addr = b.listener.local_addr().unwrap();

        let join = tokio::spawn(async move { b.wait_for_redirect(Duration::from_secs(5)).await });
        let mut s = TcpStream::connect(addr).await.expect("connect loopback");
        s.write_all(b"GET /?state=ONLY HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .unwrap();
        let err = join.await.unwrap().expect_err("must fail");
        assert!(matches!(err, OAuthError::MissingAuthorizationCode { .. }));
    }
}
