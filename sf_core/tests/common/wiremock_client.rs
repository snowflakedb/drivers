use rand::random;
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use super::file_utils;

extern crate serde_json;

const WIREMOCK_VERSION: &str = "3.13.2";
const WIREMOCK_DIR: &str = "tests/wiremock";
const WIREMOCK_JAR_SUBDIR: &str = "wiremock_standalone";
const WIREMOCK_MAPPINGS_SUBDIR: &str = "mappings";

pub struct WiremockClient {
    process: Child,
    http_port: u16,
    https_port: u16,
    host: String,
    workspace_root: PathBuf,
    stderr_path: PathBuf,
}

impl WiremockClient {
    /// Start a new Wiremock instance
    ///
    /// - Find a free port for HTTP
    /// - Start the Wiremock standalone JAR
    /// - Wait for Wiremock to be healthy
    pub fn start() -> Self {
        let workspace_root = file_utils::repo_root();
        let wiremock_dir = workspace_root.join(WIREMOCK_DIR);
        let jar_filename = format!("wiremock-standalone-{}.jar", WIREMOCK_VERSION);
        let jar_path = wiremock_dir.join(WIREMOCK_JAR_SUBDIR).join(jar_filename);

        if !jar_path.exists() {
            panic!("Wiremock JAR not found at: {}", jar_path.display());
        }

        let http_port = Self::find_free_port();
        let https_port = Self::find_free_port();

        // Create a throwaway keystore for HTTPS. This is test-only and pairs with
        // verify_certificates=false in the client TLS config (option A).
        // Use a unique ID per WireMock instance to avoid conflicts when tests run in parallel
        let unique_id = format!("{}-{:x}", std::process::id(), random::<u64>());
        let keystore_dir = env::temp_dir().join(format!("ud-wiremock-{}", unique_id));
        fs::create_dir_all(&keystore_dir).expect("Failed to create temp keystore dir");
        // Use a simple JKS keystore for Wiremock HTTPS (server mode, no MITM).
        let keystore_path = keystore_dir.join("wiremock-keystore.jks");
        let keystore_password = "changeit";

        // Generate the keystore using keytool (part of JDK).
        // NOTE: Wiremock needs a cert valid for localhost + 127.0.0.1.
        // Try multiple keytool locations since macOS /usr/bin/keytool may not work
        // without a system Java installation, but Homebrew OpenJDK has its own keytool.
        let keytool_candidates = [
            "/opt/homebrew/opt/openjdk/bin/keytool", // Homebrew Apple Silicon (most common)
            "/usr/local/opt/openjdk/bin/keytool",    // Homebrew Intel Mac
            "keytool",                               // PATH lookup (fallback)
        ];

        let mut keytool_success = false;
        let mut last_keytool_error = String::new();

        for keytool_path in keytool_candidates {
            // Remove any partial keystore from previous failed attempt
            let _ = fs::remove_file(&keystore_path);

            let result = Command::new(keytool_path)
                .arg("-genkeypair")
                .arg("-alias")
                .arg("wiremock")
                .arg("-keyalg")
                .arg("RSA")
                .arg("-keysize")
                .arg("2048")
                .arg("-dname")
                .arg("CN=localhost")
                .arg("-validity")
                .arg("3650")
                .arg("-storetype")
                .arg("JKS")
                .arg("-keystore")
                .arg(&keystore_path)
                .arg("-storepass")
                .arg(keystore_password)
                .arg("-keypass")
                .arg(keystore_password)
                .arg("-ext")
                .arg("SAN=dns:localhost,ip:127.0.0.1")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    keytool_success = true;
                    break;
                }
                Ok(output) => {
                    last_keytool_error = format!(
                        "{}: exit code {:?}, stderr: {}",
                        keytool_path,
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(e) => {
                    last_keytool_error = format!("{}: {}", keytool_path, e);
                }
            }
        }

        if !keytool_success {
            panic!(
                "keytool failed to generate HTTPS keystore for Wiremock. \
                Tried locations: {:?}. Last error: {}",
                keytool_candidates, last_keytool_error
            );
        }

        // WireMock CLI flags have varied across versions/distributions.
        // Try a small set of common flag combinations and fall back with useful diagnostics.
        let stderr_path = keystore_dir.join("wiremock-stderr.log");
        let mut last_err = None::<String>;

        let candidates: Vec<Vec<String>> = vec![
            // Newer-style: https-keystore + keystore-password
            vec![
                "--https-keystore".to_string(),
                keystore_path.display().to_string(),
                "--keystore-password".to_string(),
                keystore_password.to_string(),
                "--key-manager-password".to_string(),
                keystore_password.to_string(),
            ],
            // Common older-style: keystore + keystore-password
            vec![
                "--keystore".to_string(),
                keystore_path.display().to_string(),
                "--keystore-password".to_string(),
                keystore_password.to_string(),
                "--key-manager-password".to_string(),
                keystore_password.to_string(),
            ],
            // Alternative naming: keystore-path
            vec![
                "--keystore-path".to_string(),
                keystore_path.display().to_string(),
                "--keystore-password".to_string(),
                keystore_password.to_string(),
                "--key-manager-password".to_string(),
                keystore_password.to_string(),
            ],
            // Alternative: https-keystore-path + https-keystore-password
            vec![
                "--https-keystore-path".to_string(),
                keystore_path.display().to_string(),
                "--https-keystore-password".to_string(),
                keystore_password.to_string(),
                "--https-key-manager-password".to_string(),
                keystore_password.to_string(),
            ],
        ];

        let mut process: Option<Child> = None;
        for extra_args in candidates {
            // Truncate stderr log for each attempt.
            let _ = fs::write(&stderr_path, "");

            let mut cmd = Command::new("java");
            cmd.arg("-jar")
                .arg(&jar_path)
                .arg("--root-dir")
                .arg(&wiremock_dir)
                .arg("--port")
                .arg(http_port.to_string())
                .arg("--https-port")
                .arg(https_port.to_string());

            for a in extra_args {
                cmd.arg(a);
            }

            let stderr_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&stderr_path)
                .expect("Failed to open Wiremock stderr log file");

            let child = cmd
                .stdout(std::process::Stdio::null())
                .stderr(stderr_file)
                .spawn();

            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    last_err = Some(format!("Failed to spawn Wiremock (java): {e}"));
                    continue;
                }
            };

            // If Wiremock immediately exits (e.g., unknown flag), capture stderr and try next.
            thread::sleep(Duration::from_millis(200));
            if let Ok(Some(status)) = child.try_wait() {
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                last_err = Some(format!(
                    "Wiremock exited immediately (status {status}); stderr:\n{stderr}"
                ));
                continue;
            }

            process = Some(child);
            break;
        }

        let process = process.unwrap_or_else(|| {
            panic!(
                "Failed to start Wiremock with HTTPS enabled. Last error:\n{}",
                last_err.unwrap_or_else(|| "unknown error".to_string())
            )
        });

        let mut client = WiremockClient {
            process,
            http_port,
            https_port,
            host: "localhost".to_string(),
            workspace_root,
            stderr_path,
        };

        client.wait_for_health();

        client
    }

    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.host, self.http_port)
    }

    pub fn https_url(&self) -> String {
        format!("https://{}:{}", self.host, self.https_port)
    }

    /// Add a mapping from a JSON
    ///
    /// # Arguments
    /// * `mapping_path` - Relative path from tests/wiremock/mappings/ directory
    /// * `placeholders` - Optional map of custom placeholder strings
    ///
    pub fn add_mapping(
        &self,
        mapping_path: &str,
        placeholders: Option<&std::collections::HashMap<String, String>>,
    ) {
        let full_path = self
            .workspace_root
            .join(WIREMOCK_DIR)
            .join(WIREMOCK_MAPPINGS_SUBDIR)
            .join(mapping_path);

        if !full_path.exists() {
            panic!("Mapping file not found: {}", full_path.display());
        }

        let mut mapping_content = fs::read_to_string(&full_path).unwrap_or_else(|e| {
            panic!("Failed to read mapping file {}: {}", full_path.display(), e)
        });

        let mut all_placeholders = placeholders.cloned().unwrap_or_default();
        all_placeholders.insert(
            "{{REPO_ROOT}}".to_string(),
            self.workspace_root.to_str().unwrap().to_string(),
        );

        for (placeholder, value) in &all_placeholders {
            mapping_content = mapping_content.replace(placeholder, value);
        }

        let json: serde_json::Value = serde_json::from_str(&mapping_content).unwrap_or_else(|e| {
            panic!(
                "Invalid JSON in mapping file {}: {}",
                full_path.display(),
                e
            )
        });

        let client = reqwest::blocking::Client::new();
        let add_url = format!("{}/__admin/mappings", self.http_url());

        if let Some(mappings_array) = json.get("mappings").and_then(|m| m.as_array()) {
            for mapping in mappings_array {
                let response = client
                    .post(&add_url)
                    .header("Content-Type", "application/json")
                    .json(mapping)
                    .send()
                    .expect("Failed to send mapping request to Wiremock");

                if !response.status().is_success() {
                    panic!(
                        "Failed to add mapping, status: {}, body: {}",
                        response.status(),
                        response.text().unwrap_or_default()
                    );
                }
            }
        } else {
            let response = client
                .post(&add_url)
                .header("Content-Type", "application/json")
                .body(mapping_content)
                .send()
                .expect("Failed to send mapping request to Wiremock");

            if !response.status().is_success() {
                panic!(
                    "Failed to add mapping, status: {}, body: {}",
                    response.status(),
                    response.text().unwrap_or_default()
                );
            }
        }
    }

    /// Set a scenario to a specific state
    ///
    /// This is used to activate scenario-based mappings that require a specific state.
    /// WireMock scenarios start in "Started" state by default.
    pub fn set_scenario_state(&self, scenario_name: &str, state: &str) {
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "{}/__admin/scenarios/{}/state",
            self.http_url(),
            scenario_name
        );

        let body = serde_json::json!({ "state": state });

        let response = client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .expect("Failed to set scenario state");

        if !response.status().is_success() {
            panic!(
                "Failed to set scenario state, status: {}, body: {}",
                response.status(),
                response.text().unwrap_or_default()
            );
        }
    }

    fn wait_for_health(&mut self) {
        let max_retries = 60;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        for _attempt in 1..=max_retries {
            thread::sleep(Duration::from_millis(500));

            // If the process exited, fail fast with stderr output.
            if let Ok(Some(status)) = self.process.try_wait() {
                let stderr = fs::read_to_string(&self.stderr_path).unwrap_or_default();
                panic!(
                    "Wiremock process exited (status {status}) before becoming healthy.\nStderr:\n{stderr}"
                );
            }

            let health_url = format!("{}/__admin/health", self.http_url());
            match client.get(&health_url).send() {
                Ok(response) => {
                    if response.status().is_success()
                        && let Ok(text) = response.text()
                        && text.contains("\"status\"")
                        && text.contains("\"healthy\"")
                    {
                        return;
                    }
                }
                Err(_) => {
                    // Connection refused is expected until Wiremock starts
                    continue;
                }
            }
        }

        let stderr = fs::read_to_string(&self.stderr_path).unwrap_or_default();
        panic!(
            "Wiremock did not become healthy after {} seconds on port {}.\nStderr:\n{}",
            max_retries / 2,
            self.http_port,
            stderr
        );
    }

    fn find_free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to a free port");
        listener
            .local_addr()
            .expect("Failed to get local address")
            .port()
    }

    fn shutdown(&mut self) {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        let shutdown_url = format!("{}/__admin/shutdown", self.http_url());
        if client.post(&shutdown_url).send().is_err() {
            // If graceful shutdown fails, kill the process
            let _ = self.process.kill();
        }
    }
}

impl Drop for WiremockClient {
    fn drop(&mut self) {
        self.shutdown();
        let _ = self.process.wait();
    }
}
