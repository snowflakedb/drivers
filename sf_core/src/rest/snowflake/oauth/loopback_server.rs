//! Loopback HTTP server that receives the IdP's authorization-code redirect.
//!
//! Must bind explicitly to `127.0.0.1` (and only `::1` when the user
//! supplies a literal IPv6 redirect URI). Do **not** replicate Node's
//! `0.0.0.0` bind — see `analysis_feature_oauth.md` §3.5 and §14 #11.
//!
//! The server accepts a single connection, parses the request line for
//! `code` + `state` (or `error` + `error_description`), responds with a
//! small HTML success/error page, and returns control to the caller.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use snafu::ResultExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
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

const READ_BUFFER_BYTES: usize = 8 * 1024;

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
    /// out of the request line, send a tiny HTML success page, and return.
    ///
    /// `timeout` is the *total* deadline for accepting **and** reading the
    /// request, mirroring the browser-response-timeout knob in JDBC and ODBC.
    pub(crate) async fn wait_for_redirect(
        self,
        timeout: Duration,
    ) -> Result<RedirectResult, OAuthError> {
        let LoopbackBinding {
            listener,
            redirect_uri,
        } = self;

        let outcome = tokio::time::timeout(timeout, async move {
            let (mut socket, peer) = listener.accept().await.context(PortBindSnafu)?;
            tracing::debug!(peer = %peer, "OAuth loopback accepted browser redirect");

            let request_line = read_request_line(&mut socket).await?;
            let result = parse_request_line(&request_line, &redirect_uri);

            let body = match &result {
                Ok(_) => SUCCESS_HTML.to_string(),
                Err(_) => error_html(),
            };
            let _ = write_http_response(&mut socket, &body).await;
            let _ = socket.shutdown().await;
            result
        })
        .await;

        match outcome {
            Ok(res) => res,
            Err(_) => BrowserTimeoutSnafu.fail(),
        }
    }
}

/// Read up to the end of the HTTP request line (CRLF-terminated). We do
/// not need the headers or body — the OAuth redirect is a GET with all
/// parameters in the query string.
async fn read_request_line(socket: &mut tokio::net::TcpStream) -> Result<String, OAuthError> {
    let mut buf = Vec::with_capacity(READ_BUFFER_BYTES);
    let mut chunk = [0u8; 1024];
    loop {
        let n = socket.read(&mut chunk).await.context(PortBindSnafu)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_crlf(&buf) {
            return Ok(String::from_utf8_lossy(&buf[..pos]).into_owned());
        }
        if buf.len() >= READ_BUFFER_BYTES {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

fn parse_request_line(line: &str, redirect_uri: &Url) -> Result<RedirectResult, OAuthError> {
    // request line: "GET /path?query HTTP/1.1"
    let mut parts = line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let target_url = Url::parse(redirect_uri.as_str())
        .and_then(|base| base.join(target))
        .or_else(|_| Url::parse(&format!("http://127.0.0.1{target}")))
        .map_err(|e| OAuthError::RedirectUriParse {
            source: e,
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    let mut error_description: Option<String> = None;
    for (k, v) in target_url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            "error" => error = Some(v.into_owned()),
            "error_description" => error_description = Some(v.into_owned()),
            _ => {}
        }
    }

    if let Some(err) = error {
        return IdpSnafu {
            error: err,
            description: error_description.unwrap_or_default(),
        }
        .fail();
    }
    let Some(code) = code else {
        return MissingAuthorizationCodeSnafu.fail();
    };
    let state = state.unwrap_or_default();
    Ok(RedirectResult {
        code: SensitiveString::from(code),
        state,
    })
}

async fn write_http_response(
    socket: &mut tokio::net::TcpStream,
    body: &str,
) -> Result<(), OAuthError> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket
        .write_all(response.as_bytes())
        .await
        .context(PortBindSnafu)?;
    Ok(())
}

fn error_html() -> String {
    "<!doctype html><html><body><h1>Authorization failed.</h1><p>You may close this window and check the driver logs.</p></body></html>".to_string()
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
        // Poke the loopback like a browser would.
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
        // Don't connect. Wait for the timeout to fire.
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
