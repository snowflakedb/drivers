//! A live `mitmdump` (mitmproxy) subprocess used as an external HTTPS-MITM
//! forward proxy for the honest e2e proxy test.
//!
//! Unlike the hermetic [`crate::common::connect_proxy::ConnectProxy`] — a
//! CONNECT tunnel this repo hand-rolled from its own reading of the proxy
//! protocol — `mitmdump` is proxy software the driver team did not write. A
//! client-side misunderstanding of how a real proxy behaves therefore cannot
//! silently agree with it, which is the whole point of driving the transfer
//! through it.
//!
//! Ports snowflake-connector-python's `MitmClient`
//! (`test/test_utils/mitm/mitm_client.py`) process lifecycle: spawn with an
//! OS-assigned port, wait — with deadlines, never a bare sleep — for the CA
//! cert and the addon-reported port, verify the listener accepts connections,
//! and kill the process on `Drop` so a panicking test never leaks it. The
//! generated CA lives in a per-instance `confdir` tempdir rather than the
//! user's `~/.mitmproxy`, so the trust anchor is scoped to this proxy and
//! cleaned up automatically.
//!
//! mitmdump's stdout/stderr are redirected to files in the tempdir rather than
//! pipes: a busy PUT could otherwise fill the OS pipe buffer that nothing is
//! draining and wedge the proxy mid-transfer. On any startup failure the files
//! are read back into the panic message so the addon's own errors are visible.

// Only `e2e/put_get/proxy_live_mitm.rs` uses this module; every other test
// binary compiles `tests/common` on its own and sees these items as unused.
#![allow(dead_code)]

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::snowflake_test_client::CloudProvider;

/// The addon script, embedded at compile time so the running test never depends
/// on a source path being present on the CI runner.
const PORT_DETECTOR_ADDON: &str = include_str!("mitm_port_detector.py");

const CA_CERT_WAIT: Duration = Duration::from_secs(30);
const PORT_WAIT: Duration = Duration::from_secs(10);
const LISTEN_WAIT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Returns `true` if `mitmdump` is invokable on `PATH`. Callers gate the test on
/// this and skip with a visible message when it is `false` — the test must
/// never silently pass when the proxy binary is missing.
pub fn mitmdump_available() -> bool {
    Command::new("mitmdump")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A running `mitmdump` proxy that owns its process, config dir, and the port
/// file the addon writes.
pub struct MitmProxy {
    child: Child,
    port: u16,
    ca_cert_path: PathBuf,
    // NDJSON file the addon's `request()` hook appends one line per intercepted
    // request to; read back by `recorded_requests`.
    request_log_path: PathBuf,
    // Kept alive for the proxy's lifetime: the confdir holds the generated CA
    // the connection must trust, and the tempdir also backs the port file and
    // the redirected stdout/stderr logs. Dropping it early would invalidate
    // `ca_cert_path`.
    _dir: tempfile::TempDir,
}

impl MitmProxy {
    /// Spawn `mitmdump` on a fresh OS-assigned loopback port and block until it
    /// is ready to proxy. Panics on any startup failure — callers should have
    /// gated on [`mitmdump_available`] first.
    pub fn start() -> Self {
        let dir = tempfile::tempdir().expect("create mitmdump tempdir");
        let confdir = dir.path().join("confdir");
        std::fs::create_dir(&confdir).expect("create mitmdump confdir");
        let addon_path = dir.path().join("mitm_port_detector.py");
        std::fs::write(&addon_path, PORT_DETECTOR_ADDON).expect("write mitmdump addon");
        let port_file = dir.path().join("port.txt");
        let request_log_path = dir.path().join("requests.log");
        let stdout_path = dir.path().join("mitmdump.stdout");
        let stderr_path = dir.path().join("mitmdump.stderr");
        // mitmproxy writes its PEM-encoded CA here once it initialises.
        let ca_cert_path = confdir.join("mitmproxy-ca-cert.pem");

        let stdout = std::fs::File::create(&stdout_path).expect("create mitmdump stdout log");
        let stderr = std::fs::File::create(&stderr_path).expect("create mitmdump stderr log");

        let mut child = Command::new("mitmdump")
            .arg("--listen-host")
            .arg("127.0.0.1")
            .arg("--listen-port")
            .arg("0") // OS assigns a free port; the addon reports the real one.
            .arg("--set")
            .arg(format!("confdir={}", confdir.display()))
            .arg("--set")
            .arg("connection_strategy=lazy") // don't dial upstream until needed
            .arg("--set")
            .arg("stream_large_bodies=1m")
            .arg("-s")
            .arg(&addon_path)
            // Child-scoped env only — never the test process's env, so no other
            // test in this binary observes it (flaky-tests rule
            // `ud-no-environment-variable-side-effects`).
            .env("MITM_PORT_FILE", &port_file)
            .env("MITM_REQUEST_LOG", &request_log_path)
            // Redirect to files, not pipes: nothing drains a pipe during the
            // transfer, so a busy PUT could fill the buffer and wedge mitmdump.
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .expect("spawn mitmdump");

        let logs = MitmLogs {
            stdout: stdout_path,
            stderr: stderr_path,
        };
        wait_for_file(
            &ca_cert_path,
            CA_CERT_WAIT,
            &mut child,
            &logs,
            "CA certificate",
        );
        let port = read_port(&port_file, PORT_WAIT, &mut child, &logs);
        wait_for_listener(port, LISTEN_WAIT, &mut child, &logs);

        Self {
            child,
            port,
            ca_cert_path,
            request_log_path,
            _dir: dir,
        }
    }

    /// Loopback port to feed into `proxy_port`.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Path to mitmdump's generated CA PEM, to load into the connection's
    /// `custom_root_store_path` so the MITM'd handshake verifies.
    pub fn ca_cert_path(&self) -> &Path {
        &self.ca_cert_path
    }

    /// Every request mitmdump intercepted, in arrival order (with duplicates).
    /// Empty until the addon logs its first request, or if nothing transited.
    /// The mitm-CA-only trust store proves transit structurally; this proves it
    /// directly, by method/host/path (mirrors
    /// [`crate::common::tls_proxy::TlsProxy::received_requests`]).
    pub fn recorded_requests(&self) -> Vec<RecordedRequest> {
        std::fs::read_to_string(&self.request_log_path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

/// One request mitmdump intercepted: method, host (`pretty_host`), and path
/// (with query string). No body — `stream_large_bodies` means large parts never
/// fully buffer, and path+method already carry the multipart/ranged shape.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RecordedRequest {
    pub method: String,
    pub host: String,
    pub path: String,
}

/// Suffix of the internal-stage storage endpoint per cloud: S3
/// (`*.amazonaws.com`), Azure blob (`*.blob.core.windows.net`), GCS
/// (`storage.googleapis.com`). The Snowflake GS host (`*.snowflakecomputing.com`)
/// matches none of these, so filtering on it isolates the storage transfer from
/// the control-plane login/query traffic.
pub fn cloud_storage_host_suffix(cloud: CloudProvider) -> &'static str {
    match cloud {
        CloudProvider::Aws => "amazonaws.com",
        CloudProvider::Azure => "windows.net",
        CloudProvider::Gcp => "googleapis.com",
    }
}

impl Drop for MitmProxy {
    fn drop(&mut self) {
        // Unconditional kill: the process must not outlive the test whether it
        // passed or panicked (flaky-tests rules `ud-no-resource-leak-in-tests`
        // / `ud-teardown-must-not-depend-on-test-passing`).
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The files mitmdump's stdout/stderr are redirected to, read back into panic
/// messages so a startup failure surfaces the addon's own errors.
struct MitmLogs {
    stdout: PathBuf,
    stderr: PathBuf,
}

impl MitmLogs {
    fn dump(&self) -> String {
        let out = std::fs::read_to_string(&self.stdout).unwrap_or_default();
        let err = std::fs::read_to_string(&self.stderr).unwrap_or_default();
        format!("--- mitmdump stdout ---\n{out}--- mitmdump stderr ---\n{err}")
    }
}

/// Poll until `path` exists, the deadline passes, or mitmdump dies.
fn wait_for_file(path: &Path, timeout: Duration, child: &mut Child, logs: &MitmLogs, what: &str) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        check_alive(child, logs, what);
        if Instant::now() >= deadline {
            fail(
                child,
                logs,
                &format!("{what} not present at {}", path.display()),
                timeout,
            );
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Poll the addon's port file until it holds a parseable port.
fn read_port(port_file: &Path, timeout: Duration, child: &mut Child, logs: &MitmLogs) -> u16 {
    let deadline = Instant::now() + timeout;
    loop {
        check_alive(child, logs, "port detection");
        if let Some(port) = std::fs::read_to_string(port_file)
            .ok()
            .and_then(|c| c.trim().parse::<u16>().ok())
        {
            return port;
        }
        if Instant::now() >= deadline {
            fail(child, logs, "addon did not report a port", timeout);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Confirm the proxy actually accepts TCP connections before the test relies on
/// it — the port file is written in the `running()` hook, but this closes the
/// gap between "addon ran" and "listener is accepting".
fn wait_for_listener(port: u16, timeout: Duration, child: &mut Child, logs: &MitmLogs) {
    let deadline = Instant::now() + timeout;
    loop {
        check_alive(child, logs, "listener readiness");
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        if Instant::now() >= deadline {
            fail(
                child,
                logs,
                &format!("not accepting connections on 127.0.0.1:{port}"),
                timeout,
            );
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// If mitmdump has already exited, fail with its captured output — turns a
/// silent "waited for something that will never come" into a clear failure.
fn check_alive(child: &mut Child, logs: &MitmLogs, phase: &str) {
    if let Ok(Some(status)) = child.try_wait() {
        panic!(
            "mitmdump exited ({status}) during {phase}.\n{}",
            logs.dump()
        );
    }
}

/// Kill mitmdump and panic with its captured output — the addon logs its own
/// startup errors there, so a version-mismatch in the `running()` hook /
/// `listen_addrs()` API surfaces instead of a bare timeout.
fn fail(child: &mut Child, logs: &MitmLogs, reason: &str, timeout: Duration) -> ! {
    let _ = child.kill();
    let _ = child.wait();
    panic!("mitmdump {reason} within {timeout:?}.\n{}", logs.dump());
}
