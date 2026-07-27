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
//! - CRL Distribution Points: download and parse each CRL

use std::collections::HashMap;
use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use tracing::warn;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::*;

use crate::config::connection_config::DiagnosticConfig;
use crate::log_foreign_error;

const REPORT_FILENAME: &str = "SnowflakeConnectionTestReport.txt";
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

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
}

impl DiagnosticRunner {
    /// Create the runner.  Performs the initial SNOWFLAKE_URL probe (DNS +
    /// TLS cert inspection), resolves and validates both the log path and the
    /// allowlist path, and emits path-validation warnings via `tracing::warn!`.
    pub fn new(account: &str, host: &str, config: DiagnosticConfig) -> Self {
        let DiagnosticConfig::Enabled {
            log_path,
            allowlist_path,
        } = config
        else {
            unreachable!("DiagnosticRunner::new called with DiagnosticConfig::Disabled");
        };

        let sections = ["INITIAL", "PROXY", "SNOWFLAKE_URL", "STAGE", "IGNORE"];
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

        // Probe the main Snowflake host (DNS + peer IP + TLS cert inspection).
        probe_host(host, 443, "SNOWFLAKE_URL", &mut results);

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
    pub fn run_post_connect(&mut self, allowlist_json: Option<String>) {
        let entries = self.load_allowlist(allowlist_json);
        if let Some(entries) = entries {
            self.allowlist_retrieval_success = true;
            for entry in entries {
                let host_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let host = entry.get("host").and_then(|v| v.as_str()).unwrap_or("");
                let port: u16 = entry.get("port").and_then(|v| v.as_u64()).unwrap_or(443) as u16;
                if host_type == "STAGE" {
                    probe_host(host, port, "STAGE", &mut self.results);
                }
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
            let stage = join_section(&self.results, "STAGE");
            msg.push_str(&format!(
                "\n=========Snowflake Stage information===================================\n\
                 We retrieved stage info from the allowlist\n\
                 {stage}\n"
            ));
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
///   - port 443 → TLS handshake + cert inspection
///   - port 80  → HTTP GET connectivity check
///   - other    → TCP-only success/fail log
fn probe_host(host: &str, port: u16, section: &str, results: &mut HashMap<String, Vec<String>>) {
    dns_lookup(host, section, results);

    let addr = format!("{host}:{port}");
    let sock_addr = match std::net::ToSocketAddrs::to_socket_addrs(&addr.as_str()) {
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

    match TcpStream::connect_timeout(&sock_addr, PROBE_TIMEOUT) {
        Ok(stream) => {
            // Log the actual connected IP — mirrors gosnowflake's conn.RemoteAddr() logging.
            if let Ok(peer) = stream.peer_addr() {
                let ip = peer.ip();
                append(
                    results,
                    section,
                    &format!("{host}:{port}: Connected to IP: {ip}"),
                );
            }

            if port == 443 {
                inspect_tls(host, stream, section, results);
            } else if port == 80 {
                do_http_check(host, port, stream, section, results);
            } else {
                append(
                    results,
                    section,
                    &format!("{host}:{port}: URL Check: Connected Successfully"),
                );
            }
        }
        Err(e) => {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: {e}"),
            );
        }
    }
}

/// Perform TLS handshake over `stream` and inspect the certificate chain.
///
/// Mirrors gosnowflake's `doHTTPSGetCerts`: logs serial (hex), subject,
/// issuer, validity, crt.sh link, and CRL Distribution Points for each cert.
fn inspect_tls(
    host: &str,
    stream: TcpStream,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
) {
    // Build TLS client config with system root certificates.
    let root_store = {
        let mut native = rustls_native_certs::load_native_certs();
        let mut store = rustls::RootCertStore::empty();
        // Drain any load errors silently — diagnostic should not fail.
        native.errors.clear();
        store.add_parsable_certificates(native.certs);
        store
    };

    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(),
    );

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

    let conn = match ClientConnection::new(config, server_name) {
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

                // CRL Distribution Points — download and parse each.
                let crl_urls: Vec<String> = cert
                    .extensions()
                    .iter()
                    .filter_map(|ext| {
                        if let ParsedExtension::CRLDistributionPoints(points) =
                            ext.parsed_extension()
                        {
                            Some(points.points.iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .filter_map(|point| point.distribution_point.as_ref())
                    .filter_map(|name| {
                        if let x509_parser::extensions::DistributionPointName::FullName(names) =
                            name
                        {
                            Some(names.iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .filter_map(|gn| {
                        if let GeneralName::URI(uri) = gn {
                            Some(uri.to_string())
                        } else {
                            None
                        }
                    })
                    .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
                    .collect();

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
                        fetch_and_parse_crl(crl_url, host, cert_num, section, results);
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
/// Mirrors gosnowflake's `fetchCRL`.
fn fetch_and_parse_crl(
    url: &str,
    host: &str,
    cert_num: usize,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
) {
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

    // Check for HTTP 200.
    let header = String::from_utf8_lossy(&response[..body_start]);
    let status_line = header.lines().next().unwrap_or("");
    if !status_line.contains("200") {
        return Err(format!("HTTP error: {status_line}"));
    }

    Ok(response[body_start..].to_vec())
}

/// HTTP GET connectivity check over an already-established TCP stream (port 80).
fn do_http_check(
    host: &str,
    port: u16,
    mut stream: TcpStream,
    section: &str,
    results: &mut HashMap<String, Vec<String>>,
) {
    let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_ok() {
        let mut buf = [0u8; 4096];
        let response = match stream.read(&mut buf) {
            Ok(n) => String::from_utf8_lossy(&buf[..n]).to_string(),
            Err(_) => String::new(),
        };
        let ok_patterns = ["200", "301", "cloudfront"];
        if ok_patterns.iter().any(|p| response.contains(p)) || response.is_empty() {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Connected Successfully"),
            );
        } else {
            append(
                results,
                section,
                &format!("{host}:{port}: URL Check: Failed: {response}"),
            );
        }
    } else {
        append(
            results,
            section,
            &format!("{host}:{port}: URL Check: Connected Successfully"),
        );
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
