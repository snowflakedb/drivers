//! Loopback HTTP server that receives the IdP's authorization-code redirect.
//!
//! Must bind explicitly to `127.0.0.1` (and only `::1` when the user
//! supplies a literal IPv6 redirect URI). Do **not** replicate Node's
//! `0.0.0.0` bind — it widens the OAuth-window attack surface to the
//! local network.
//!
//! The bind decision and the hostname we *advertise* to the IdP in
//! `redirect_uri` are deliberately split. The listener always sits on
//! a loopback IP literal (`127.0.0.1` or `::1`), but the advertised
//! `redirect_uri` round-trips the user-supplied hostname when it is a
//! loopback equivalent (`localhost`, any `127.0.0.0/8` literal, `::1`).
//! Several IdPs — Okta among them — perform exact-string matching on
//! the registered redirect URI per OAuth 2.0 §3.1.2.3, so rewriting
//! `http://localhost:.../cb` to `http://127.0.0.1:.../cb` would
//! silently break valid configurations even though the two strings
//! address the same socket. See [`loopback_host_string_from_hint`] for
//! the exact policy.
//!
//! HTTP parsing and response writing are delegated to `axum`; we keep
//! ownership of the bind decision and the single-shot semantics
//! (oneshot channel + graceful shutdown).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
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

/// Returns the host string to round-trip into the advertised
/// `redirect_uri` when the user supplied a hint, or `None` if the hint
/// host is not a safe loopback equivalent.
///
/// Safe loopback equivalents are:
///
/// * the literal `"localhost"` (case-insensitive — RFC 4343);
/// * any IPv4 loopback literal (the full `127.0.0.0/8` block per
///   RFC 1122 §3.2.1.3, e.g. `127.0.0.1`, `127.0.0.2`);
/// * the IPv6 loopback literal `::1`.
///
/// Anything else (a non-loopback IP literal that was already coerced
/// to a loopback bind by [`bind`], or an arbitrary domain name) returns
/// `None` so the caller falls back to the IP literal of the bound
/// socket. That fallback is what keeps a misconfigured
/// `oauth_redirect_uri=http://attacker.example/...` from being
/// propagated to the IdP — it still gets rewritten to `127.0.0.1`,
/// just like before this helper existed.
fn loopback_host_string_from_hint(hint: &Url) -> Option<String> {
    match hint.host()? {
        url::Host::Domain(d) if d.eq_ignore_ascii_case("localhost") => Some(d.to_string()),
        url::Host::Ipv4(v4) if v4.is_loopback() => Some(v4.to_string()),
        url::Host::Ipv6(v6) if v6.is_loopback() => Some(format!("[{v6}]")),
        _ => None,
    }
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
///
/// The hostname surfaced in the returned `redirect_uri` is computed
/// separately from the bind decision — see
/// [`loopback_host_string_from_hint`] — so a `localhost`-based hint
/// round-trips to the IdP unchanged even though the listener itself
/// always sits on a loopback IP literal.
pub(crate) async fn bind(redirect_uri_hint: Option<&Url>) -> Result<LoopbackBinding, OAuthError> {
    let (ip, port, path) = match redirect_uri_hint {
        Some(url) => {
            // Hard-enforce the cross-driver invariant: only ever bind a
            // loopback interface, even if the supplied `oauth_redirect_uri`
            // carries a different host (do not replicate Node's `0.0.0.0`
            // bind). A misconfigured (or malicious) hint must not widen the
            // listener's exposure beyond the local machine.
            let host = url.host();
            let ip = match host {
                Some(url::Host::Ipv4(v4)) if v4.is_loopback() => IpAddr::V4(v4),
                Some(url::Host::Ipv6(v6)) if v6.is_loopback() => IpAddr::V6(v6),
                Some(url::Host::Ipv4(_)) => {
                    tracing::warn!(
                        "OAuth redirect_uri specified non-loopback IPv4 host; binding to 127.0.0.1 instead"
                    );
                    IpAddr::V4(Ipv4Addr::LOCALHOST)
                }
                Some(url::Host::Ipv6(_)) => {
                    tracing::warn!(
                        "OAuth redirect_uri specified non-loopback IPv6 host; binding to ::1 instead"
                    );
                    IpAddr::V6(Ipv6Addr::LOCALHOST)
                }
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

    // Preserve the user-supplied hostname in the advertised redirect URI
    // when it's a loopback equivalent (see [`loopback_host_string_from_hint`])
    // so IdPs that do exact-string redirect_uri matching (OAuth 2.0
    // §3.1.2.3 — e.g. Okta) accept the request. Falling back to the IP
    // literal of the bound socket keeps the security override above
    // intact for non-loopback hints — those are still advertised as
    // `127.0.0.1` / `[::1]` regardless of what the caller passed in.
    let host_for_url = redirect_uri_hint
        .and_then(loopback_host_string_from_hint)
        .unwrap_or_else(|| match local.ip() {
            IpAddr::V4(_) => "127.0.0.1".to_string(),
            IpAddr::V6(_) => "[::1]".to_string(),
        });
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
        assert_eq!(b.redirect_uri.host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.port(), Some(port));
        assert_eq!(b.redirect_uri.path(), "/cb");
    }

    #[tokio::test]
    async fn bind_with_localhost_and_explicit_port_uses_exact_port() {
        // The two contract guarantees for `oauth_redirect_uri`:
        //   1. When a port is given (e.g. `http://localhost:8001/cb`),
        //      the listener binds that exact port on the loopback
        //      interface so it can be registered ahead of time with
        //      the IdP.
        //   2. When the user supplies `localhost`, the advertised
        //      `redirect_uri` round-trips that hostname (regression
        //      fix; previously rewritten to `127.0.0.1`).
        //
        // Picks a free port by binding `:0` first, then immediately
        // releasing — racy in theory but stable enough for unit tests
        // and faster than a fixed-port collision recovery loop.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let hint =
            Url::parse(&format!("http://localhost:{port}/snowflake/oauth-redirect")).unwrap();
        let b = bind(Some(&hint)).await.expect("bind on localhost:<port>");
        let bound = b.listener.local_addr().unwrap();
        assert!(
            bound.ip().is_loopback(),
            "listener must bind a loopback interface, got {bound}",
        );
        assert_eq!(
            bound.port(),
            port,
            "listener must honor the explicit hint port",
        );
        assert_eq!(b.redirect_uri.host_str(), Some("localhost"));
        assert_eq!(b.redirect_uri.port(), Some(port));
        assert_eq!(b.redirect_uri.path(), "/snowflake/oauth-redirect");
    }

    #[tokio::test]
    async fn bind_without_hint_assigns_ephemeral_loopback_port() {
        // Explicit contract: when no `oauth_redirect_uri` is configured,
        // the listener binds `127.0.0.1:0` so the kernel picks any
        // free ephemeral port. The advertised `redirect_uri` echoes
        // that assigned port back so the IdP can redirect to it.
        let b = bind(None).await.expect("bind without hint");
        let bound = b.listener.local_addr().unwrap();
        assert!(
            bound.ip().is_loopback(),
            "default bind must be on a loopback interface, got {bound}",
        );
        assert_ne!(bound.port(), 0, "kernel must have assigned a real port");
        assert_eq!(b.redirect_uri.host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.port(), Some(bound.port()));
    }

    #[tokio::test]
    async fn bind_preserves_localhost_hostname_in_redirect_uri() {
        // Regression: the listener used to advertise `127.0.0.1` to the
        // IdP even when the caller supplied `localhost`, which broke
        // OAuth 2.0 §3.1.2.3 exact-string redirect_uri matching (Okta
        // and friends). The bind itself stays on a loopback interface;
        // only the advertised hostname round-trips the user input.
        let hint = Url::parse("http://localhost:0/snowflake/oauth-redirect").unwrap();
        let b = bind(Some(&hint)).await.expect("bind on localhost hint");
        assert_eq!(
            b.redirect_uri.host_str(),
            Some("localhost"),
            "redirect_uri must round-trip the user-supplied localhost hostname",
        );
        assert_eq!(b.redirect_uri.path(), "/snowflake/oauth-redirect");
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "listener must still bind a loopback interface, got {addr}",
        );
    }

    #[tokio::test]
    async fn bind_preserves_localhost_hostname_case_insensitively() {
        // `url::Url` normalises the host to lowercase already, but
        // exercise the match arm explicitly so a future change to
        // `url`'s normalisation behaviour cannot regress us silently.
        let hint = Url::parse("http://LocalHost:0/cb").unwrap();
        let b = bind(Some(&hint)).await.expect("bind on LocalHost hint");
        assert_eq!(b.redirect_uri.host_str(), Some("localhost"));
    }

    #[tokio::test]
    async fn bind_preserves_non_canonical_ipv4_loopback_literal() {
        // The whole `127.0.0.0/8` block is loopback per RFC 1122; users
        // who registered (say) `127.0.0.2` with their IdP must see the
        // exact literal echoed back, not the canonical `127.0.0.1`.
        // Some CI containers refuse the `127.0.0.2` bind — skip rather
        // than fail in that case so the regression test stays useful
        // on developer machines without flaking CI.
        let hint = Url::parse("http://127.0.0.2:0/cb").unwrap();
        let Ok(b) = bind(Some(&hint)).await else {
            eprintln!("skipping: 127.0.0.2 bind unavailable on this host");
            return;
        };
        assert_eq!(
            b.redirect_uri.host_str(),
            Some("127.0.0.2"),
            "redirect_uri must round-trip non-canonical IPv4 loopback literal",
        );
    }

    #[tokio::test]
    async fn localhost_hint_end_to_end_redirect_succeeds() {
        // Belt-and-braces: bind on `localhost`, then send a fake IdP
        // redirect to the bound loopback socket. The handler must
        // still pick up the code/state even though `redirect_uri`
        // advertises `localhost` rather than the IP literal.
        let hint = Url::parse("http://localhost:0/cb").unwrap();
        let b = bind(Some(&hint)).await.expect("bind on localhost hint");
        assert_eq!(b.redirect_uri.host_str(), Some("localhost"));
        let addr = b.listener.local_addr().unwrap();

        let join = tokio::spawn(async move { b.wait_for_redirect(Duration::from_secs(5)).await });
        let mut s = TcpStream::connect(addr).await.expect("connect loopback");
        s.write_all(b"GET /cb?code=LH-CODE&state=LH-STATE HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let res = join.await.unwrap().expect("wait_for_redirect");
        assert_eq!(res.code.reveal(), "LH-CODE");
        assert_eq!(res.state, "LH-STATE");
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
    async fn bind_with_non_loopback_ipv4_hint_falls_back_to_loopback() {
        // The listener must always bind a loopback
        // interface, even when the user-supplied `oauth_redirect_uri`
        // carries a non-loopback host (mistake or attack). 192.0.2.0/24 is
        // the RFC 5737 documentation block — it is guaranteed not to be
        // configured on any interface, so attempting to bind it directly
        // would fail with EADDRNOTAVAIL. We rely on the coercion instead.
        let hint = Url::parse("http://192.0.2.1:0/cb").unwrap();
        let b = bind(Some(&hint)).await.expect("bind must succeed");
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "non-loopback hint must be coerced to loopback, got {}",
            addr.ip()
        );
        assert_eq!(b.redirect_uri.host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.path(), "/cb");
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

    #[tokio::test]
    async fn bind_with_non_loopback_ipv6_hint_falls_back_to_loopback() {
        // The literal example called out in the task. 2001:db8::/32 is
        // the RFC 3849 documentation prefix and is guaranteed not to be
        // assigned on any local interface, so a naive bind would fail
        // with EADDRNOTAVAIL. We deliberately fall back to a loopback
        // bind — the resulting socket must still be
        // on a loopback interface.
        let hint = Url::parse("http://[2001:db8::1]:0/cb").expect("parse hint");
        let Ok(b) = bind(Some(&hint)).await else {
            eprintln!("skipping: IPv6 loopback unavailable on this host");
            return;
        };
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "non-loopback ipv6 hint must be coerced to loopback, got {addr}"
        );
        assert!(
            matches!(b.redirect_uri.host_str(), Some("127.0.0.1") | Some("[::1]")),
            "redirect_uri host must be loopback, got {:?}",
            b.redirect_uri.host_str()
        );
        assert_eq!(b.redirect_uri.path(), "/cb", "path must be preserved");
    }

    #[tokio::test]
    async fn ipv6_loopback_hint_binds_to_ipv6_loopback() {
        // Positive complement to the test above: `[::1]` is the only
        // IPv6 address we honor explicitly. Validates the §3.5 wording
        // that we may bind `::1` "only if the user-supplied redirect
        // URI is literal IPv6".
        let hint = Url::parse("http://[::1]:0/cb").expect("parse hint");
        let Ok(b) = bind(Some(&hint)).await else {
            // Some CI hosts disable IPv6 entirely (e.g. dual-stack
            // turned off in the kernel). Skip rather than fail.
            eprintln!("skipping: IPv6 loopback unavailable on this host");
            return;
        };
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "ipv6 loopback hint must still produce a loopback bind, got {addr}"
        );
    }
}
