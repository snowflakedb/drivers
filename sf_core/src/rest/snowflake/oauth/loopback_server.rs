//! Loopback HTTP server that receives the IdP's authorization-code redirect.
//!
//! Always binds a loopback interface (`127.0.0.1`, or `::1` for an explicit
//! IPv6 URI). The bind address and the URI advertised to the IdP are kept
//! separate — see [`bind`] and [`loopback_host_string_from_hint`].

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
use tracing::instrument::WithSubscriber;
use url::Url;

use super::error::{
    BrowserTimeoutSnafu, IdpSnafu, MissingAuthorizationCodeSnafu, OAuthError, PortBindSnafu,
    RedirectUriParseSnafu,
};
use crate::config::configured_redirect_uri::ConfiguredRedirectUri;
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

/// Bound loopback listener + resolved redirect URI.
///
/// Use `.redirect_uri.parsed()` for axum route matching and the browser
/// launcher; use `.redirect_uri.as_configured()` as the string forwarded to
/// the IdP. See [`bind`] for the verbatim-vs-canonical policy.
pub(crate) struct LoopbackBinding {
    listener: TcpListener,
    pub(crate) redirect_uri: ConfiguredRedirectUri,
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

/// Returns the host string to use in the advertised `redirect_uri` when the
/// hint host is a safe loopback equivalent, or `None` otherwise.
///
/// Safe loopback equivalents:
/// * `"localhost"` (case-insensitive — RFC 4343)
/// * any IPv4 loopback literal (`127.0.0.0/8`, RFC 1122 §3.2.1.3)
/// * the IPv6 loopback literal `::1`
///
/// Returning `None` causes [`bind`] to fall back to the IP literal of the
/// bound socket, preventing non-loopback hints from being propagated to the IdP.
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
/// * If `redirect_uri_hint` is `Some`, binds that host/port and preserves the
///   path (defaults to `/`). Otherwise binds `127.0.0.1:0` (ephemeral).
/// * Always binds a loopback interface: `127.0.0.1` by default, `::1` only
///   for an explicit IPv6 literal hint.
/// * The advertised string in the returned [`LoopbackBinding`] is the
///   verbatim hint when the hint carries an explicit non-zero port on a
///   loopback-equivalent host; otherwise it is the canonical reconstructed
///   URI. See [`loopback_host_string_from_hint`] for the host classification.
pub(crate) async fn bind(
    redirect_uri_hint: Option<&ConfiguredRedirectUri>,
) -> Result<LoopbackBinding, OAuthError> {
    let hint_url = redirect_uri_hint.map(ConfiguredRedirectUri::parsed);

    let (ip, port, path) = match hint_url {
        Some(url) => {
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
    // Classify the hint host once: `Some` iff it's a loopback equivalent we
    // can safely echo back to the IdP (drives both the advertised host and
    // the verbatim-vs-canonical decision below).
    let loopback_host = hint_url.and_then(loopback_host_string_from_hint);
    let host_for_url = loopback_host.clone().unwrap_or_else(|| match local.ip() {
        IpAddr::V4(_) => "127.0.0.1".to_string(),
        IpAddr::V6(_) => "[::1]".to_string(),
    });
    let canonical = Url::parse(&format!("http://{host_for_url}:{}{path}", local.port()))
        .context(RedirectUriParseSnafu)?;

    let hint_has_explicit_nonzero_port = hint_url.and_then(|u| u.port()).is_some_and(|p| p != 0);
    let hint_is_loopback_equivalent = loopback_host.is_some();

    // Verbatim when hint has an explicit non-zero port on a loopback-equivalent
    // host; canonical reconstructed URI otherwise (ephemeral port, non-loopback
    // coercion, or no hint).
    let redirect_uri = if hint_has_explicit_nonzero_port && hint_is_loopback_equivalent {
        let raw = redirect_uri_hint
            .expect("hint present when explicit-port condition is true")
            .as_configured()
            .to_string();
        ConfiguredRedirectUri::from_parts(canonical, raw)
    } else {
        let raw = canonical.to_string();
        ConfiguredRedirectUri::from_parts(canonical, raw)
    };

    tracing::debug!(
        bound = %local,
        redirect_uri_authority = %redirect_uri.parsed().authority(),
        redirect_uri_path = %redirect_uri.parsed().path(),
        redirect_uri_advertised = %redirect_uri.as_configured(),
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
            .route(redirect_uri.parsed().path(), get(handle_redirect))
            .with_state(state);

        let server_task = tokio::spawn(
            async move {
                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = shutdown_rx.await;
                    })
                    .await;
            }
            .with_current_subscriber(),
        );

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

    fn hint(raw: &str) -> ConfiguredRedirectUri {
        ConfiguredRedirectUri::parse(raw).expect("valid test hint URL")
    }

    #[tokio::test]
    async fn bind_uses_loopback_ipv4_by_default() {
        let b = bind(None).await.expect("bind");
        let addr = b.listener.local_addr().unwrap();
        match addr.ip() {
            IpAddr::V4(v4) => assert!(v4.is_loopback(), "non-loopback ipv4 bind: {v4}"),
            IpAddr::V6(_) => panic!("default bind should be ipv4 loopback"),
        }
        assert_eq!(b.redirect_uri.parsed().scheme(), "http");
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("127.0.0.1"));
        assert!(b.redirect_uri.parsed().port().is_some());
    }

    #[tokio::test]
    async fn bind_honors_hint_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let h = hint(&format!("http://127.0.0.1:{port}/cb"));
        let b = bind(Some(&h)).await.expect("bind on hint port");
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.parsed().port(), Some(port));
        assert_eq!(b.redirect_uri.parsed().path(), "/cb");
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

        let h = hint(&format!("http://localhost:{port}/snowflake/oauth-redirect"));
        let b = bind(Some(&h)).await.expect("bind on localhost:<port>");
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
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("localhost"));
        assert_eq!(b.redirect_uri.parsed().port(), Some(port));
        assert_eq!(b.redirect_uri.parsed().path(), "/snowflake/oauth-redirect");
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
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.parsed().port(), Some(bound.port()));
    }

    #[tokio::test]
    async fn bind_preserves_localhost_hostname_in_redirect_uri() {
        // Regression: the listener used to advertise `127.0.0.1` to the
        // IdP even when the caller supplied `localhost`, which broke
        // OAuth 2.0 §3.1.2.3 exact-string redirect_uri matching (Okta
        // and friends). The bind itself stays on a loopback interface;
        // only the advertised hostname round-trips the user input.
        let h = hint("http://localhost:0/snowflake/oauth-redirect");
        let b = bind(Some(&h)).await.expect("bind on localhost hint");
        assert_eq!(
            b.redirect_uri.parsed().host_str(),
            Some("localhost"),
            "redirect_uri must round-trip the user-supplied localhost hostname",
        );
        assert_eq!(b.redirect_uri.parsed().path(), "/snowflake/oauth-redirect");
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
        let h = hint("http://LocalHost:0/cb");
        let b = bind(Some(&h)).await.expect("bind on LocalHost hint");
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("localhost"));
    }

    #[tokio::test]
    async fn bind_preserves_non_canonical_ipv4_loopback_literal() {
        // The whole `127.0.0.0/8` block is loopback per RFC 1122; users
        // who registered (say) `127.0.0.2` with their IdP must see the
        // exact literal echoed back, not the canonical `127.0.0.1`.
        // Some CI containers refuse the `127.0.0.2` bind — skip rather
        // than fail in that case so the regression test stays useful
        // on developer machines without flaking CI.
        let h = hint("http://127.0.0.2:0/cb");
        let Ok(b) = bind(Some(&h)).await else {
            eprintln!("skipping: 127.0.0.2 bind unavailable on this host");
            return;
        };
        assert_eq!(
            b.redirect_uri.parsed().host_str(),
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
        let h = hint("http://localhost:0/cb");
        let b = bind(Some(&h)).await.expect("bind on localhost hint");
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("localhost"));
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
        let h = hint("http://192.0.2.1:0/cb");
        let b = bind(Some(&h)).await.expect("bind must succeed");
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "non-loopback hint must be coerced to loopback, got {}",
            addr.ip()
        );
        assert_eq!(b.redirect_uri.parsed().host_str(), Some("127.0.0.1"));
        assert_eq!(b.redirect_uri.parsed().path(), "/cb");
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
        let h = hint("http://[2001:db8::1]:0/cb");
        let Ok(b) = bind(Some(&h)).await else {
            eprintln!("skipping: IPv6 loopback unavailable on this host");
            return;
        };
        let addr = b.listener.local_addr().unwrap();
        assert!(
            addr.ip().is_loopback(),
            "non-loopback ipv6 hint must be coerced to loopback, got {addr}"
        );
        assert!(
            matches!(
                b.redirect_uri.parsed().host_str(),
                Some("127.0.0.1") | Some("[::1]")
            ),
            "redirect_uri host must be loopback, got {:?}",
            b.redirect_uri.parsed().host_str()
        );
        assert_eq!(
            b.redirect_uri.parsed().path(),
            "/cb",
            "path must be preserved"
        );
    }

    #[tokio::test]
    async fn ipv6_loopback_hint_binds_to_ipv6_loopback() {
        // Positive complement to the test above: `[::1]` is the only
        // IPv6 address we honor explicitly. Validates the §3.5 wording
        // that we may bind `::1` "only if the user-supplied redirect
        // URI is literal IPv6".
        let h = hint("http://[::1]:0/cb");
        let Ok(b) = bind(Some(&h)).await else {
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

    // ─── Verbatim advertised URI tests (Approach A regression coverage) ───

    #[tokio::test]
    async fn explicit_port_loopback_hint_advertises_verbatim_no_trailing_slash() {
        // Core regression: `http://localhost:12346` must be advertised
        // to the IdP WITHOUT a trailing slash. `url::Url` canonicalises
        // the empty path to `/`; `redirect_uri_advertised` must use the
        // verbatim user string instead.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let raw = format!("http://localhost:{port}");
        let h = hint(&raw);
        let b = bind(Some(&h)).await.expect("bind");
        assert_eq!(
            b.redirect_uri.as_configured(),
            raw,
            "advertised URI must be verbatim (no trailing slash added)"
        );
    }

    #[tokio::test]
    async fn explicit_port_loopback_hint_with_trailing_slash_advertises_verbatim() {
        // When the user explicitly supplies the trailing slash, it must
        // survive round-trip — advertised exactly as configured.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let raw = format!("http://localhost:{port}/");
        let h = hint(&raw);
        let b = bind(Some(&h)).await.expect("bind");
        assert_eq!(b.redirect_uri.as_configured(), raw);
    }

    #[tokio::test]
    async fn explicit_port_loopback_hint_with_path_advertises_verbatim() {
        // Path is preserved verbatim when an explicit port is given.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let raw = format!("http://localhost:{port}/cb");
        let h = hint(&raw);
        let b = bind(Some(&h)).await.expect("bind");
        assert_eq!(b.redirect_uri.as_configured(), raw);
    }

    #[tokio::test]
    async fn ephemeral_hint_falls_back_to_canonical_advertised_uri() {
        // Ephemeral `:0` hint: kernel assigns a real port so the user's
        // string (containing `:0`) is NOT usable. Advertised URI must
        // be the reconstructed canonical form with the real port.
        let h = hint("http://localhost:0/cb");
        let b = bind(Some(&h)).await.expect("bind");
        let bound_port = b.listener.local_addr().unwrap().port();
        assert_ne!(bound_port, 0);
        assert_eq!(
            b.redirect_uri.as_configured(),
            format!("http://localhost:{bound_port}/cb"),
            "ephemeral hint must fall back to canonical URI with assigned port"
        );
    }

    #[tokio::test]
    async fn no_hint_advertises_canonical_loopback_uri() {
        // No hint: advertised URI is the reconstructed canonical form
        // (127.0.0.1 + assigned ephemeral port).
        let b = bind(None).await.expect("bind");
        let bound_port = b.listener.local_addr().unwrap().port();
        assert_eq!(
            b.redirect_uri.as_configured(),
            format!("http://127.0.0.1:{bound_port}/"),
        );
    }

    #[tokio::test]
    async fn non_loopback_hint_advertises_coerced_loopback_uri() {
        // Non-loopback hint is coerced to 127.0.0.1 for security; the
        // advertised URI must also reflect the coerced host, not the
        // original non-loopback address.
        let h = hint("http://192.0.2.1:0/cb");
        let b = bind(Some(&h)).await.expect("bind must succeed");
        assert_eq!(
            b.listener.local_addr().unwrap().ip().to_string(),
            "127.0.0.1"
        );
        assert!(
            b.redirect_uri
                .as_configured()
                .starts_with("http://127.0.0.1:"),
            "non-loopback hint must advertise coerced 127.0.0.1, got: {}",
            b.redirect_uri.as_configured()
        );
    }

    #[tokio::test]
    async fn explicit_port_verbatim_hint_end_to_end_redirect_succeeds() {
        // End-to-end: bind using a verbatim hint (explicit port, no trailing
        // slash), send a fake IdP redirect, and confirm the handler fires
        // on the parsed path while the advertised string is preserved.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let raw = format!("http://localhost:{port}");
        let h = hint(&raw);
        let b = bind(Some(&h)).await.expect("bind");

        // Axum routes on the parsed path ("/"); advertised stays verbatim.
        assert_eq!(b.redirect_uri.as_configured(), raw);
        assert_eq!(b.redirect_uri.parsed().path(), "/");

        let addr = b.listener.local_addr().unwrap();
        let join = tokio::spawn(async move { b.wait_for_redirect(Duration::from_secs(5)).await });
        let mut s = TcpStream::connect(addr).await.expect("connect loopback");
        s.write_all(b"GET /?code=VB-CODE&state=VB-STATE HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let res = join.await.unwrap().expect("wait_for_redirect");
        assert_eq!(res.code.reveal(), "VB-CODE");
        assert_eq!(res.state, "VB-STATE");
    }
}
