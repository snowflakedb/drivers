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
    host: String,
    workspace_root: PathBuf,
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

        let process = Command::new("java")
            .arg("-jar")
            .arg(&jar_path)
            .arg("--root-dir")
            .arg(&wiremock_dir)
            .arg("--enable-browser-proxying") // work as forward proxy
            .arg("--proxy-pass-through")
            .arg("false") // pass through only matched requests
            .arg("--port")
            .arg(http_port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to start Wiremock process");

        let client = WiremockClient {
            process,
            http_port,
            host: "localhost".to_string(),
            workspace_root,
        };

        client.wait_for_health();

        client
    }

    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.host, self.http_port)
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

    fn wait_for_health(&self) {
        let max_retries = 60;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        for _attempt in 1..=max_retries {
            thread::sleep(Duration::from_millis(500));

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

        panic!(
            "Wiremock did not become healthy after {} seconds on port {}",
            max_retries / 2,
            self.http_port
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
