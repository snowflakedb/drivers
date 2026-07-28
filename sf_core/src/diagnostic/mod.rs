//! SnowCD-style connectivity diagnostic runner.
//!
//! When `enable_connection_diag=true`, [`DiagnosticRunner`] probes DNS and
//! TCP connectivity, processes an allowlist, and writes
//! `SnowflakeConnectionTestReport.txt` to disk.  The report format mirrors
//! the reference `snowflake-connector-python` implementation.
//!
//! gosnowflake checklist implemented here:
//! - DNS resolution with private/public IP classification
//! - Actual connected peer IP (not just DNS, mirrors `conn.RemoteAddr()`)
//! - PrivateLink: flag when a `.privatelink.` host resolves to a public IP
//! - TLS certificate chain: serial, subject, issuer, validity, crt.sh link
//! - CRL Distribution Points: download and parse each CRL, deduped per run
//! - All allowlist entry types (not just STAGE), dispatching on port
//! - Proxy inheritance: proxy honored on all ports when configured (CONNECT
//!   tunnel for 443/other, absolute-form GET for 80)
//! - HTTP status: integer set {200, 301, 302, 307, 308, 400, 403}

use std::collections::{HashMap, HashSet};
use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};
use tracing::warn;
use x509_parser::prelude::*;

use crate::config::connection_config::DiagnosticConfig;
use crate::config::rest_parameters::ClientInfo;
use crate::log_foreign_error;
use crate::tls::config::ProxyConfig;

const REPORT_FILENAME: &str = "SnowflakeConnectionTestReport.txt";
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// HTTP status codes that indicate successful connectivity (mirrors gosnowflake).
///
/// 200 = success, 400 = GCS/Azure bare GET, 403 = S3 bare GET.
/// 3xx redirects would be followed by Go's http.Client; with raw TCP we treat
/// them as connectivity proof instead of following them.
///
/// Behavior change vs. the prior `["200", "301", "cloudfront"]` substring match:
/// this is now a strict status-code set, so e.g. a `404` (even from a
/// CloudFront-fronted endpoint that previously matched on the `cloudfront`
/// substring) is reported as a failure. Empty / unparseable responses still
/// count as connectivity success (see `do_http_check`).
const ACCEPTABLE_HTTP_STATUS: &[u16] = &[200, 301, 302, 307, 308, 400, 403];

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Collects and writes a connectivity diagnostic report.
pub struct DiagnosticRunner {
    allowlist_path: Option<PathBuf>,
    /// Per-section message buckets.
    results: HashMap<String, Vec<String>>,
    /// Set to `true` once the allowlist is successfully retrieved.
    allowlist_retrieval_success: bool,
    /// Resolved log directory (after path validation / fallback).
    log_dir: PathBuf,
    /// rustls config built from the connection's TlsConfig (same root store,
    /// CRL verifier, and TLS version window as the main connection).
    tls_client_config: Arc<rustls::ClientConfig>,
    /// Proxy settings from the main connection (used for CONNECT tunneling on
    /// port-443/other probes and absolute-form GET on port-80 probes).
    proxy_config: ProxyConfig,
    /// CRL URLs already fetched *successfully* this run — prevents re-fetching
    /// the same CRL from multiple certs in the same chain.  Failed fetches are
    /// not recorded, so they are retried per cert.  Scoped per run (not global)
    /// to avoid the gosnowflake process-global staleness bug.
    tested_crls: HashSet<String>,
    /// Allowlist entry types seen, in first-appearance order.  Used to emit
    /// all entry-type sections in the report rather than STAGE only.
    allowlist_sections: Vec<String>,
    /// Seen-set for O(1) dedup in `run_post_connect`.  Mirrors `allowlist_sections`.
    allowlist_sections_seen: HashSet<String>,
}

impl DiagnosticRunner {
    /// Create the runner.  Performs the initial SNOWFLAKE_URL probe (DNS +
    /// TLS cert inspection), resolves and validates both the log path and the
    /// allowlist path, and emits path-validation warnings via `tracing::warn!`.
    pub fn new(
        account: &str,
        host: &str,
        config: DiagnosticConfig,
        tls_client_config: Arc<rustls::ClientConfig>,
        proxy_config: ProxyConfig,
        client_info: &ClientInfo,
    ) -> Self {
        let DiagnosticConfig::Enabled {
            log_path,
            allowlist_path,
        } = config
        else {
            unreachable!("DiagnosticRunner::new called with DiagnosticConfig::Disabled");
        };

        let sections = ["INITIAL", "PROXY", "SNOWFLAKE_URL"];
        let mut results: HashMap<String, Vec<String>> = sections
            .iter()
            .map(|s| (s.to_string(), Vec::new()))
            .collect();

        // ---- INITIAL section -----------------------------------------------
        append(
            &mut results,
            "INITIAL",
            &format!("Specified snowflake account: {account}"),
        );
        append(
            &mut results,
            "INITIAL",
            &format!("Host based on specified account: {host}"),
        );

        for line in collect_environment_info(client_info) {
            append(&mut results, "INITIAL", &line);
        }

        // Probe the main Snowflake host (DNS + peer IP + TLS cert inspection).
        // Initialize tested_crls here so CRL URLs seen in the Snowflake host cert
        // chain are remembered and not re-fetched for post-connect allowlist entries.
        let mut tested_crls: HashSet<String> = HashSet::new();
        probe_host(
            host,
            443,
            "SNOWFLAKE_URL",
            &mut results,
            &tls_client_config,
            &proxy_config,
            &mut tested_crls,
        );

        // ---- Resolve log directory -----------------------------------------
        let tmpdir: PathBuf = std::env::temp_dir();
        let log_dir = match log_path {
            None => tmpdir.clone(),
            Some(p) => {
                if !p.is_absolute() {
                    warn!("Path {} for connection test is not absolute.", p.display());
                    tmpdir.clone()
                } else if !p.exists() {
                    warn!("Path {} for connection test does not exist.", p.display());
                    tmpdir.clone()
                } else {
                    p
                }
            }
        };

        // ---- Validate allowlist path ----------------------------------------
        if let Some(ref ap) = allowlist_path {
            if !ap.is_absolute() {
                warn!(
                    "Path '{}' for connection test allowlist is not absolute.",
                    ap.display()
                );
                warn!(
                    "Will connect to Snowflake for allowlist json instead.  \
                     If you did not provide a valid password, please make sure \
                     to update and run again."
                );
            } else if !ap.exists() {
                warn!(
                    "File '{}' for connection test allowlist does not exist.",
                    ap.display()
                );
                warn!(
                    "Will connect to Snowflake for allowlist json instead.  \
                     If you did not provide a valid password, please make sure \
                     to update and run again."
                );
            }
        }

        Self {
            allowlist_path,
            results,
            allowlist_retrieval_success: false,
            log_dir,
            tls_client_config,
            proxy_config,
            tested_crls,
            allowlist_sections: Vec::new(),
            allowlist_sections_seen: HashSet::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Public interface
    // -----------------------------------------------------------------------

    /// Run pre-connect diagnostics: proxy detection.
    pub fn run_pre_connect(&mut self) {
        self.check_proxies();
    }

    /// Run post-connect diagnostics: load allowlist and probe each entry.
    ///
    /// `allowlist_json` is the raw `system$allowlist()` response from an
    /// already-established session.  Pass `None` when the connection failed
    /// (the file-path branch is tried automatically from `config.allowlist_path`).
    ///
    /// All entry types are probed (not just STAGE), dispatching on port exactly
    /// as gosnowflake does.
    pub fn run_post_connect(&mut self, allowlist_json: Option<String>) {
        let entries = self.load_allowlist(allowlist_json);
        if let Some(entries) = entries {
            self.allowlist_retrieval_success = true;
            // Clone what we need to avoid simultaneous mutable + immutable borrows
            // on different fields of `self` across the loop.
            let tls_config = Arc::clone(&self.tls_client_config);
            let proxy_config = self.proxy_config.clone();
            for entry in entries {
                let host_type = entry
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let host = entry
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let port: u16 = entry.get("port").and_then(|v| v.as_u64()).unwrap_or(443) as u16;

                // Track section order for the report (seen-set gives O(1) dedup).
                if self.allowlist_sections_seen.insert(host_type.clone()) {
                    self.allowlist_sections.push(host_type.clone());
                }

                probe_host(
                    &host,
                    port,
                    &host_type,
                    &mut self.results,
                    &tls_config,
                    &proxy_config,
                    &mut self.tested_crls,
                );
            }
        }
    }

    /// Assemble and write the report.  Also emits the full report at DEBUG via
    /// `tracing::debug!` so the Python `caplog` fixture can capture it.
    pub fn write_report(&self) {
        let message = self.build_report();
        tracing::debug!("{}", message);
        let path = self.log_dir.join(REPORT_FILENAME);
        if let Err(e) = std::fs::write(&path, &message) {
            warn!(
                "Failed to write diagnostic report to {}: {e}",
                path.display()
            );
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn check_proxies(&mut self) {
        // Remove proxy env vars temporarily to get system proxy info.
        let proxy_keys = ["HTTP_PROXY", "HTTPS_PROXY", "https_proxy", "http_proxy"];
        let mut saved: Vec<(String, String)> = Vec::new();
        for key in proxy_keys {
            if let Ok(val) = std::env::var(key) {
                saved.push((key.to_string(), val));
                // SAFETY: modifying env in a single-threaded diagnostic path.
                unsafe { std::env::remove_var(key) };
            }
        }

        let system_proxies = collect_proxy_env();
        append(
            &mut self.results,
            "PROXY",
            &format!("Proxies with Env vars removed(SYSTEM PROXIES): {system_proxies}"),
        );

        // Restore env vars.
        for (key, val) in &saved {
            unsafe { std::env::set_var(key, val) };
        }

        let env_proxies = collect_proxy_env();
        append(
            &mut self.results,
            "PROXY",
            &format!("Proxies with Env vars restored(ENV PROXIES): {env_proxies}"),
        );
    }

    /// Try to load the allowlist from the pre-fetched file, then from the SQL
    /// result, then give up and record a message.
    fn load_allowlist(
        &mut self,
        sql_result: Option<String>,
    ) -> Option<Vec<HashMap<String, serde_json::Value>>> {
        // File path takes priority.
        if let Some(ref path) = self.allowlist_path
            && path.is_absolute()
            && path.exists()
        {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    return parse_allowlist(&content, &mut self.results);
                }
                Err(e) => {
                    append(
                        &mut self.results,
                        "INITIAL",
                        &format!(
                            "Allowlist was not valid json: '{e}'.  \
                             Please run 'select system$allowlist();' and validate \
                             the file {path} is correct.",
                            path = path.display()
                        ),
                    );
                    return None;
                }
            }
        }

        // Fall back to SQL result if available.
        if let Some(json) = sql_result {
            return parse_allowlist(&json, &mut self.results);
        }

        // Nothing to work with.
        None
    }

    fn build_report(&self) -> String {
        let initial = join_section(&self.results, "INITIAL");
        let proxy = join_section(&self.results, "PROXY");
        let sf_url = join_section(&self.results, "SNOWFLAKE_URL");

        let mut msg = format!(
            "=========Connectivity diagnostic report================================\n\
             {initial}\n\
             \n\
             =========Proxy information - These are best guesses, not guarantees====\n\
             {proxy}\n\
             \n\
             =========Snowflake URL information=====================================\n\
             {sf_url}\n"
        );

        if self.allowlist_retrieval_success {
            // Keep existing section title so tests that assert on the exact string pass.
            msg.push_str(
                "\n=========Snowflake Stage information===================================\n\
                 We retrieved stage info from the allowlist\n",
            );
            // Emit results for every entry type in first-appearance order.
            for section in &self.allowlist_sections {
                let content = join_section(&self.results, section);
                if !content.is_empty() {
                    msg.push_str(&content);
                    msg.push('\n');
                }
            }
        } else {
            msg.push_str(
                "\n=========Snowflake Stage information - Unavailable=====================\n\
                 We could not connect to Snowflake to get allowlist, so we do not have stage\n\
                 diagnostic info\n",
            );
        }

        msg
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn append(results: &mut HashMap<String, Vec<String>>, section: &str, msg: &str) {
    results
        .entry(section.to_string())
        .or_default()
        .push(format!("{section}: {msg}"));
}

fn join_section(results: &HashMap<String, Vec<String>>, section: &str) -> String {
    results
        .get(section)
        .map(|lines| lines.join("\n"))
        .unwrap_or_default()
}

/// Perform a DNS lookup for `host` and append nslookup result lines.
/// If the host is a PrivateLink endpoint that resolves to a public IP, logs an error.
fn dns_lookup(host: &str, section: &str, results: &mut HashMap<String, Vec<String>>) {
    let is_privatelink = host.contains(".privatelink.");
    match std::net::ToSocketAddrs::to_socket_addrs(&(host, 443u16)) {
        Ok(addrs) => {
            let ips: Vec<std::net::IpAddr> = addrs.map(|a| a.ip()).collect();
            let mut seen = std::collections::HashSet::new();
            for ip in ips {
                if seen.insert(ip) {
                    if ip.is_loopback() || is_private_ip(&ip) {
                        append(
                            results,
                            section,
                            &format!("{host}: nslookup results: private ip: {ip}"),
                        );
                    } else {
                        append(
                            results,
                            section,
                            &format!("{host}: nslookup results: public ip: {ip}"),
                        );
                        if is_privatelink {
                            append(
                                results,
                                section,
                                &format!(
                                    "{host}: PrivateLink host resolved to public IP {ip} — \
                                     check DNS configuration"
                                ),
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            log_foreign_error!(warn, e, "Connectivity Test Exception in list_ips");
        }
    }
}

fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_multicast(),
    }
}

/// Probe `host:port`: DNS lookup, TCP connect (logs actual peer IP), then
/// port-specific check:
///   - port 443 → TLS handshake + cert inspection (via proxy CONNECT when configured)
///   - port 80  → HTTP GET connectivity check with integer status validation
///     (absolute-form request URI through the proxy when configured)
///   - other    → reachability check (proxy CONNECT tunnel when configured, else TCP-only)
///
/// When an explicit proxy is configured it is honored for every port, mirroring
/// gosnowflake copying the proxy from its transport factory: the TCP connection
/// is made to the proxy and the target is reached via CONNECT (443/other) or an
/// absolute-form GET (80).
fn probe_host(
    host: &str,
    port: u16,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
    tls_client_config: &Arc<rustls::ClientConfig>,
    proxy_config: &ProxyConfig,
    tested_crls: &mut HashSet<String>,
) {
    dns_lookup(host, section, results);

    // With an explicit proxy configured, make the TCP connection to the proxy
    // (for any port) and reach the target through it below.
    //
    // Note: proxies configured via HTTP_PROXY / HTTPS_PROXY environment
    // variables (use_proxy_env=true, no explicit host) are not honored here
    // because raw TCP probes cannot reuse reqwest's env-var detection.  The
    // PROXY section of the report already shows the detected env-var proxies.
    // Extract proxy host; a present-but-empty value is misconfigured — warn and ignore.
    let proxy_host: Option<&str> = match proxy_config.host.as_deref() {
        Some(h) if !h.is_empty() => Some(h),
        Some(_) => {
            warn!("{host}:{port}: Proxy host is set but empty; ignoring proxy for this probe.");
            None
        }
        None => None,
    };

    // Pre-build the Proxy-Authorization header once so both the CONNECT tunnel
    // and the absolute-form HTTP GET path can include it without duplicating
    // the base64 encoding.
    let proxy_auth_header: Option<String> = if proxy_host.is_some() {
        proxy_config
            .user
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|user| {
                let pass = proxy_config
                    .password
                    .as_ref()
                    .map(|p| p.reveal().as_str())
                    .unwrap_or("");
                format!(
                    "Proxy-Authorization: Basic {}\r\n",
                    BASE64.encode(format!("{user}:{pass}"))
                )
            })
    } else {
        None
    };
    let (tcp_host, tcp_port) = if let Some(ph) = proxy_host {
        let raw_port = proxy_config.port.unwrap_or(8080);
        let pp = u16::try_from(raw_port).unwrap_or_else(|_| {
            warn!(
                "{host}:{port}: Proxy port {raw_port} is out of range; falling back to 8080 for this probe."
            );
            8080
        });
        (ph.to_string(), pp)
    } else {
        (host.to_string(), port)
    };

    let addr_str = format!("{tcp_host}:{tcp_port}");
    let sock_addr = match std::net::ToSocketAddrs::to_socket_addrs(addr_str.as_str()) {
        Ok(mut a) => match a.next() {
            Some(s) => s,
            None => {
                append(
                    results,
                    section,
                    &format!("{host}:{port}: URL Check: Failed: could not resolve address"),
                );
                return;
            }
        },
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: {e}"),
            );
            return;
        }
    };

    let stream = match TcpStream::connect_timeout(&sock_addr, PROBE_TIMEOUT) {
        Ok(s) => s,
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: {e}"),
            );
            return;
        }
    };

    // Bound every subsequent read/write: `connect_timeout` covers only the
    // connect, so without these a stalled peer or proxy would hang this
    // blocking probe thread indefinitely (the CONNECT tunnel and TLS handshake
    // both read from the socket). Applied once here so all paths inherit it.
    stream.set_read_timeout(Some(PROBE_TIMEOUT)).ok();
    stream.set_write_timeout(Some(PROBE_TIMEOUT)).ok();

    // Log the actual connected IP — mirrors gosnowflake's conn.RemoteAddr() logging.
    // Under a proxy this is the proxy's IP, so label it to avoid confusion.
    if let Ok(peer) = stream.peer_addr() {
        let via = if proxy_host.is_some() {
            " (via proxy)"
        } else {
            ""
        };
        append(
            results,
            section,
            &format!("{host}:{port}: Connected to IP: {}{via}", peer.ip()),
        );
    }

    if port == 443 {
        // If going through a proxy, establish a CONNECT tunnel first so the TLS
        // handshake (and cert chain) reflects the real target, not the proxy.
        let stream = if proxy_host.is_some() {
            match connect_proxy_tunnel(stream, host, port, proxy_auth_header.as_deref()) {
                Ok(s) => s,
                Err(e) => {
                    append(
                        results,
                        section,
                        &format!("{host}:{port}: URL Check: Failed: proxy CONNECT: {e}"),
                    );
                    return;
                }
            }
        } else {
            stream
        };
        inspect_tls(
            host,
            stream,
            section,
            results,
            tls_client_config,
            tested_crls,
        );
    } else if port == 80 {
        // Plain HTTP: a proxy forwards it via an absolute-form request URI (no
        // CONNECT needed), so pass `use_proxy` down to shape the request line.
        do_http_check(
            host,
            port,
            stream,
            section,
            results,
            proxy_host.is_some(),
            proxy_auth_header.as_deref(),
        );
    } else if proxy_host.is_some() {
        // Non-HTTP(S) port through a proxy: the only way to prove reachability
        // is a CONNECT tunnel. A 200 from the proxy means the target accepted.
        match connect_proxy_tunnel(stream, host, port, proxy_auth_header.as_deref()) {
            Ok(_) => append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Connected Successfully"),
            ),
            Err(e) => append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: proxy CONNECT: {e}"),
            ),
        }
    } else {
        append(
            results,
            section,
            &format!("{host}:{port}: URL Check: Connected Successfully"),
        );
    }
}

/// Send an HTTP/1.1 CONNECT request over `stream` to establish a tunnel to
/// `target_host:target_port`.  Returns the same stream (now tunneled) on success.
///
/// `proxy_auth_header` is an optional pre-built `"Proxy-Authorization: Basic …\r\n"`
/// line; pass `Some(...)` when the proxy requires credentials.
fn connect_proxy_tunnel(
    mut stream: TcpStream,
    target_host: &str,
    target_port: u16,
    proxy_auth_header: Option<&str>,
) -> Result<TcpStream, String> {
    let auth = proxy_auth_header.unwrap_or("");
    let req = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\n\
         Host: {target_host}:{target_port}\r\n\
         {auth}\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;

    // Read until end-of-headers marker.
    let mut response: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                response.push(byte[0]);
                if response.ends_with(b"\r\n\r\n") {
                    break;
                }
                if response.len() > 4096 {
                    return Err("proxy CONNECT response too large".to_string());
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    let response_str = String::from_utf8_lossy(&response);
    let status_line = response_str.lines().next().unwrap_or("");
    let proxy_status: Option<u16> = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok());
    if proxy_status != Some(200) {
        return Err(format!("proxy rejected CONNECT: {status_line}"));
    }

    Ok(stream)
}

/// Perform TLS handshake over `stream` and inspect the certificate chain.
///
/// Mirrors gosnowflake's `doHTTPSGetCerts`: logs serial (hex), subject,
/// issuer, validity, crt.sh link, and CRL Distribution Points for each cert.
/// Uses the connection's TLS config (root store, CRL verifier, version window)
/// rather than re-building from system roots.
fn inspect_tls(
    host: &str,
    stream: TcpStream,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
    tls_client_config: &Arc<rustls::ClientConfig>,
    tested_crls: &mut HashSet<String>,
) {
    let server_name = match ServerName::try_from(host.to_owned()) {
        Ok(n) => n,
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}:443: TLS: invalid server name: {e}"),
            );
            return;
        }
    };

    let conn = match ClientConnection::new(Arc::clone(tls_client_config), server_name) {
        Ok(c) => c,
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}:443: TLS: connection init failed: {e}"),
            );
            return;
        }
    };

    let mut tls = StreamOwned::new(conn, stream);

    // Send a minimal HTTP/1.0 GET to complete the TLS handshake.
    let req = format!("GET / HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    if let Err(e) = tls.write_all(req.as_bytes()) {
        append(
            results,
            section,
            &format!("{host}:443: TLS: handshake failed: {e}"),
        );
        return;
    }

    append(
        results,
        section,
        &format!("{host}:443: URL Check: Connected Successfully"),
    );

    // Negotiated TLS protocol version and cipher suite — available after the
    // handshake completes. This exceeds old-driver coverage (gosnowflake and
    // Python diagnostic do not log these fields).
    if let Some(proto) = tls.conn.protocol_version() {
        append(
            results,
            section,
            &format!("{host}:443: TLS: negotiated protocol: {proto:?}"),
        );
    }
    if let Some(suite) = tls.conn.negotiated_cipher_suite() {
        append(
            results,
            section,
            &format!("{host}:443: TLS: negotiated cipher suite: {suite:?}"),
        );
    }

    // Collect peer certificate DER bytes before consuming the stream.
    let cert_ders: Vec<Vec<u8>> = tls
        .conn
        .peer_certificates()
        .map(|chain| chain.iter().map(|c| c.to_vec()).collect())
        .unwrap_or_default();

    for (idx, cert_der) in cert_ders.iter().enumerate() {
        let cert_num = idx + 1;
        match X509Certificate::from_der(cert_der) {
            Ok((_, cert)) => {
                let serial_hex = hex::encode(cert.raw_serial());
                let subject = cert.subject().to_string();
                let issuer = cert.issuer().to_string();
                let not_before = cert.validity().not_before;
                let not_after = cert.validity().not_after;

                append(
                    results,
                    section,
                    &format!(
                        "{host}: Certificate {cert_num}: serial={serial_hex}, \
                         subject={subject}, issuer={issuer}, \
                         valid={not_before} to {not_after}, \
                         crt.sh: https://crt.sh/?serial={serial_hex}"
                    ),
                );

                // CRL Distribution Points — download and parse each, with dedup.
                let crl_urls =
                    crate::crl::extract_crl_distribution_points(cert_der).unwrap_or_default();

                if crl_urls.is_empty() {
                    append(
                        results,
                        section,
                        &format!("{host}: Certificate {cert_num}: no CRL Distribution Points"),
                    );
                } else {
                    for crl_url in &crl_urls {
                        append(
                            results,
                            section,
                            &format!("{host}: Certificate {cert_num}: CRL DP: {crl_url}"),
                        );
                        fetch_and_parse_crl(crl_url, host, cert_num, section, results, tested_crls);
                    }
                }
            }
            Err(e) => {
                append(
                    results,
                    section,
                    &format!("{host}: Certificate {cert_num}: parse error: {e}"),
                );
            }
        }
    }
}

/// Fetch a CRL over HTTP (plain TCP, not TLS) and log its metadata.
///
/// Deduplicates by URL: once a CRL has been fetched *successfully* this run its
/// URL is recorded in `tested_crls` and later certs sharing the same DP skip the
/// redundant fetch.  Failed fetches are not recorded, so a transient failure is
/// retried by the next cert.  Scoped per run rather than process-globally to
/// avoid stale cached results across runs.
fn fetch_and_parse_crl(
    url: &str,
    host: &str,
    cert_num: usize,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
    tested_crls: &mut HashSet<String>,
) {
    if tested_crls.contains(url) {
        tracing::debug!(
            "{host}: Certificate {cert_num}: CRL {url}: already checked this run, skipping"
        );
        return;
    }

    // Only handle plain HTTP CRL DPs (HTTPS CRL DPs are rare).
    if !url.starts_with("http://") {
        append(
            results,
            section,
            &format!("{host}: Certificate {cert_num}: CRL {url}: skipped (non-HTTP scheme)"),
        );
        return;
    }

    match http_get_binary(url) {
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}: Certificate {cert_num}: CRL {url}: fetch failed: {e}"),
            );
        }
        Ok(body) => {
            match x509_parser::revocation_list::CertificateRevocationList::from_der(&body) {
                Err(e) => {
                    append(
                        results,
                        section,
                        &format!(
                            "{host}: Certificate {cert_num}: CRL {url}: \
                             parse failed: {e}"
                        ),
                    );
                }
                Ok((_, crl)) => {
                    let issuer = crl.tbs_cert_list.issuer.to_string();
                    let this_update = crl.tbs_cert_list.this_update.to_string();
                    let next_update = crl
                        .tbs_cert_list
                        .next_update
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "none".to_string());
                    let revoked = crl.iter_revoked_certificates().count();
                    append(
                        results,
                        section,
                        &format!(
                            "{host}: Certificate {cert_num}: CRL {url}: \
                             issuer={issuer}, thisUpdate={this_update}, \
                             nextUpdate={next_update}, revokedCount={revoked}"
                        ),
                    );
                    // Record after successful fetch so a later cert in the same chain
                    // with the same DP skips the redundant fetch.
                    tested_crls.insert(url.to_string());
                }
            }
        }
    }
}

/// Minimal HTTP/1.0 GET that returns the response body bytes.
/// Used for CRL fetching over plain HTTP.
fn http_get_binary(url: &str) -> Result<Vec<u8>, String> {
    // Strip scheme to get host+path.
    let without_scheme = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("not an http:// URL: {url}"))?;
    let (authority, path) = match without_scheme.find('/') {
        Some(pos) => (&without_scheme[..pos], &without_scheme[pos..]),
        None => (without_scheme, "/"),
    };
    let (host, port) = match authority.rfind(':') {
        Some(pos) => {
            let p: u16 = authority[pos + 1..]
                .parse()
                .map_err(|e| format!("bad port: {e}"))?;
            (&authority[..pos], p)
        }
        None => (authority, 80u16),
    };

    let sock_addr = std::net::ToSocketAddrs::to_socket_addrs(&(host, port))
        .map_err(|e| format!("resolve error: {e}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))?;

    let mut stream =
        TcpStream::connect_timeout(&sock_addr, PROBE_TIMEOUT).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(PROBE_TIMEOUT))
        .map_err(|e| e.to_string())?;

    let request = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write error: {e}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("read error: {e}"))?;

    // Find header/body separator.
    let body_start = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .ok_or_else(|| "no HTTP header separator in response".to_string())?;

    // Require HTTP 200 for CRL responses (mirrors gosnowflake's fetchCRL).
    let header = String::from_utf8_lossy(&response[..body_start]);
    let status_line = header.lines().next().unwrap_or("");
    if !status_line.contains("200") {
        return Err(format!("HTTP error: {status_line}"));
    }

    Ok(response[body_start..].to_vec())
}

/// HTTP GET connectivity check over an already-established TCP stream (port 80).
///
/// Rewrites the path to `/ocsp_response_cache.json` for OCSP cache hosts
/// (mirrors gosnowflake's hostname-prefix check).  Accepts integer status codes
/// in `ACCEPTABLE_HTTP_STATUS` rather than fragile string-pattern matching.
///
/// When `via_proxy` is set the stream is connected to the proxy (not the
/// target), so the request uses an absolute-form request URI that the proxy
/// forwards to the target.  The caller is responsible for setting read/write
/// timeouts on `stream`.
fn do_http_check(
    host: &str,
    port: u16,
    mut stream: TcpStream,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
    via_proxy: bool,
    proxy_auth_header: Option<&str>,
) {
    // OCSP cache hosts serve the cache file at a specific path.
    let path = if host.starts_with("ocsp.snowflakecomputing.") {
        "/ocsp_response_cache.json"
    } else {
        "/"
    };

    // Origin-form (`/path`) for a direct connection; absolute-form
    // (`http://host:port/path`) so an HTTP proxy forwards to the target.
    let request_target = if via_proxy {
        format!("http://{host}:{port}{path}")
    } else {
        path.to_string()
    };

    let auth = if via_proxy {
        proxy_auth_header.unwrap_or("")
    } else {
        ""
    };
    let request =
        format!("GET {request_target} HTTP/1.1\r\nHost: {host}\r\n{auth}Connection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        // TCP connected but the write failed (e.g. immediate RST from the
        // server-side socket). Count as connectivity proven at the TCP layer.
        append(
            results,
            section,
            &format!("{host}:{port}: URL Check: Connected Successfully"),
        );
        return;
    }

    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).unwrap_or(0);
    let response = String::from_utf8_lossy(&buf[..n]);

    // Extract the integer status code from the response line.
    let status: Option<u16> = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok());

    match status {
        Some(code) if ACCEPTABLE_HTTP_STATUS.contains(&code) => {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Connected Successfully"),
            );
        }
        Some(code) => {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: HTTP {code}"),
            );
        }
        None => {
            // No parseable status (empty response or non-HTTP).  TCP connection
            // succeeded so treat as connectivity success.
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Connected Successfully"),
            );
        }
    }
}

/// Parse `content` as a JSON array of objects.  Returns `None` and appends an
/// error message if the content is not a valid list of JSON objects.
fn parse_allowlist(
    content: &str,
    results: &mut HashMap<String, Vec<String>>,
) -> Option<Vec<HashMap<String, serde_json::Value>>> {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(serde_json::Value::Array(arr)) => {
            let objects: Vec<HashMap<String, serde_json::Value>> = arr
                .into_iter()
                .filter_map(|v| {
                    if let serde_json::Value::Object(m) = v {
                        Some(m.into_iter().collect())
                    } else {
                        None
                    }
                })
                .collect();
            Some(objects)
        }
        Ok(_) | Err(_) => {
            append(
                results,
                "INITIAL",
                "Allowlist is not a valid list of json objects. \
                 Please run 'select system$allowlist();' and provide as a json file \
                 using the connection_diag_allowlist_path option.",
            );
            None
        }
    }
}

/// Collect environment and driver metadata for the INITIAL report section
/// from the already-built [`ClientInfo`].
///
/// `ClientInfo` carries the same fields that the old drivers send as
/// `CLIENT_ENVIRONMENT` in every login request, so we reuse that struct
/// rather than re-detecting the same information a second time.
fn collect_environment_info(info: &ClientInfo) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push(format!("sf_core version: {}", env!("CARGO_PKG_VERSION")));
    lines.push(format!("Driver: {}", info.client_app_id));
    lines.push(format!("Driver version: {}", info.version));
    lines.push(format!("Application: {}", info.application));
    lines.push(format!("OS: {}", info.os));
    lines.push(format!("Architecture: {}", std::env::consts::ARCH));
    lines.push(format!("OS version: {}", info.os_version));

    if let Some(details) = &info.os_details
        && !details.is_empty()
    {
        let mut pairs: Vec<_> = details.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        let s = pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("OS details: {s}"));
    }

    if let Some(mode) = &info.ocsp_mode {
        lines.push(format!("OCSP mode: {mode}"));
    }

    let mode_str = match info.crl_config.check_mode {
        crate::crl::config::CertRevocationCheckMode::Disabled => "DISABLED",
        crate::crl::config::CertRevocationCheckMode::Enabled => "ENABLED",
        crate::crl::config::CertRevocationCheckMode::Advisory => "ADVISORY",
    };
    lines.push(format!("Cert revocation check mode: {mode_str}"));

    if info.platforms.is_empty() {
        lines.push("Detected platforms: (none)".to_string());
    } else {
        lines.push(format!("Detected platforms: {}", info.platforms.join(", ")));
    }

    if let Some(name) = &info.runtime_name {
        lines.push(format!("Runtime: {name}"));
    }
    if let Some(ver) = &info.runtime_version {
        lines.push(format!("Runtime version: {ver}"));
    }
    if let Some(compiler) = &info.compiler {
        lines.push(format!("Compiler: {compiler}"));
    }

    lines
}

/// Collect proxy settings from environment variables as a Python-style dict
/// string.  Returns `"{}"` when no proxy vars are set (the common case).
fn collect_proxy_env() -> String {
    let keys = [
        ("HTTPS_PROXY", "https"),
        ("HTTP_PROXY", "http"),
        ("https_proxy", "https"),
        ("http_proxy", "http"),
    ];
    let mut seen_schemes = std::collections::HashSet::new();
    let mut entries: Vec<String> = Vec::new();
    for (env_key, scheme) in keys {
        if seen_schemes.contains(scheme) {
            continue;
        }
        if let Ok(val) = std::env::var(env_key)
            && !val.is_empty()
        {
            seen_schemes.insert(scheme);
            entries.push(format!("'{scheme}': '{val}'"));
        }
    }
    if entries.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", entries.join(", "))
    }
}
