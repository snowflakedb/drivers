import json
import os
import socket
import subprocess
import tempfile
import time

from pathlib import Path

import requests

from .utils import repo_root


WIREMOCK_VERSION = "3.13.2"
WIREMOCK_DIR = "tests/wiremock"
WIREMOCK_JAR_SUBDIR = "wiremock_standalone"
WIREMOCK_MAPPINGS_SUBDIR = "mappings"
WIREMOCK_KEYSTORE = "wiremock-keystore.p12"
WIREMOCK_KEYSTORE_PASSWORD = "password"

# Protocols disabled in all modes; per-version tests extend this list.
_TLS_BASE_DISABLED = "SSLv3, TLSv1, TLSv1.1"
_TLS_VERSION_EXTRA_DISABLED = {
    "tls12": ", TLSv1.3",  # server speaks only 1.2 — disable 1.3
    "tls13": ", TLSv1.2",  # server speaks only 1.3 — disable 1.2
}


class WiremockClient:
    def __init__(self, tls_version: str | None = None):
        """Create a WiremockClient.

        Args:
            tls_version: When set to ``"tls12"`` or ``"tls13"``, the WireMock
                JVM is started with an HTTPS listener that accepts only that
                protocol version. Use ``https_url()`` to connect to it.
        """
        if tls_version is not None and tls_version not in _TLS_VERSION_EXTRA_DISABLED:
            raise ValueError(f"tls_version must be 'tls12' or 'tls13', got: {tls_version!r}")
        self.tls_version = tls_version
        self.process: subprocess.Popen | None = None
        self.http_port: int | None = None
        self.https_port: int | None = None
        self.host: str = "localhost"
        self.workspace_root: Path | None = None
        self._security_props_file: str | None = None

    def start(self) -> "WiremockClient":
        """Start a new Wiremock instance.

        - Find a free port for HTTP (and HTTPS when tls_version is set)
        - Start the Wiremock standalone JAR
        - Wait for Wiremock to be healthy
        """
        self.workspace_root = repo_root()
        wiremock_dir = self.workspace_root / WIREMOCK_DIR
        jar_path = wiremock_dir / WIREMOCK_JAR_SUBDIR / f"wiremock-standalone-{WIREMOCK_VERSION}.jar"

        if not jar_path.exists():
            raise FileNotFoundError(f"Wiremock JAR not found at: {jar_path}")

        self.http_port = self._find_free_port()

        cmd = [
            "java",
        ]

        if self.tls_version is not None:
            self.https_port = self._find_free_port()
            # Write a JVM security-properties override that restricts the server
            # to the requested TLS version. Single = appends to platform defaults
            # rather than replacing them (double == would replace everything).
            disabled = _TLS_BASE_DISABLED + _TLS_VERSION_EXTRA_DISABLED[self.tls_version]
            with tempfile.NamedTemporaryFile(mode="w", suffix=".properties", prefix="wiremock-tls-", delete=False) as f:
                f.write(f"jdk.tls.disabledAlgorithms={disabled}\n")
                self._security_props_file = f.name
            cmd += [f"-Djava.security.properties={self._security_props_file}"]

        cmd += [
            "-jar",
            str(jar_path),
            "--root-dir",
            str(wiremock_dir),
            "--enable-browser-proxying",  # work as forward proxy
            "--proxy-pass-through",
            "false",  # pass through only matched requests
            "--port",
            str(self.http_port),
        ]

        if self.tls_version is not None:
            keystore_path = wiremock_dir / WIREMOCK_KEYSTORE
            cmd += [
                "--https-port",
                str(self.https_port),
                "--https-keystore",
                str(keystore_path),
                "--keystore-type",
                "PKCS12",
                "--keystore-password",
                WIREMOCK_KEYSTORE_PASSWORD,
            ]

        # Discard JVM stdout/stderr — nothing reads these pipes, and Windows pipe
        # buffers are small (4–8 KB), so a chatty Wiremock log can fill the buffer
        # and stall the JVM's logging thread (and via it, the HTTP server thread).
        # See investigation note in SNOW-3487070.
        self.process = subprocess.Popen(
            cmd,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        self._wait_for_health()
        if self.tls_version is not None:
            self._wait_for_https_health()
        return self

    def http_url(self) -> str:
        """Get the HTTP URL for connecting to this Wiremock instance.

        Returns:
            HTTP URL string (e.g., "http://localhost:12345")
        """
        return f"http://{self.host}:{self.http_port}"

    def https_url(self) -> str:
        """Get the HTTPS URL for connecting to this Wiremock instance.

        Only valid when the client was created with a ``tls_version``.

        Returns:
            HTTPS URL string (e.g., "https://localhost:12346")
        """
        if self.https_port is None:
            raise RuntimeError("https_url() requires WiremockClient(tls_version=...)")
        return f"https://{self.host}:{self.https_port}"

    def add_mapping(self, mapping_path: str, placeholders: dict[str, str] | None = None) -> None:
        """Add a mapping to Wiremock with optional placeholder replacement.
        Args:
            mapping_path: Relative path to mapping file from wiremock/mappings/ directory
                         (e.g., "auth/login_success_jwt.json")
            placeholders: Optional dictionary of custom placeholder replacements
                         (e.g., {"{{KEY}}": "value"})
        """
        if placeholders is None:
            placeholders = {}

        mappings_dir = self.workspace_root / WIREMOCK_DIR / WIREMOCK_MAPPINGS_SUBDIR
        mapping_file = mappings_dir / mapping_path

        if not mapping_file.exists():
            raise FileNotFoundError(f"Mapping file not found: {mapping_file}")

        # Read and inject placeholders
        content = mapping_file.read_text()

        all_placeholders = {
            **placeholders,
            # Use POSIX separators so the substituted path is valid JSON on Windows
            # (backslashes in Windows paths produce invalid \escape sequences).
            "{{REPO_ROOT}}": self.workspace_root.as_posix(),
        }

        for placeholder, value in all_placeholders.items():
            content = content.replace(placeholder, value)

        # Parse the mapping
        mapping_json = json.loads(content)

        # Add each mapping via Wiremock admin API
        admin_url = f"{self.http_url()}/__admin/mappings"

        # Handle both single mapping and mappings array
        if "mappings" in mapping_json and isinstance(mapping_json["mappings"], list):
            # File contains an array of mappings - send each individually
            for mapping in mapping_json["mappings"]:
                response = requests.post(admin_url, json=mapping, timeout=5)

                if response.status_code not in (200, 201):
                    raise RuntimeError(f"Failed to add mapping: {response.status_code} {response.text}")
        else:
            # Single mapping - send the entire content as-is
            response = requests.post(admin_url, data=content, headers={"Content-Type": "application/json"}, timeout=5)

            if response.status_code not in (200, 201):
                raise RuntimeError(f"Failed to add mapping: {response.status_code} {response.text}")

    def reset(self) -> None:
        """Clear all mappings and captured requests via the admin API.

        Removes every stub (including those auto-loaded from the mappings/
        directory on disk), resets scenario state machines, and clears the
        request journal so each test starts with a blank slate.

        WireMock's ``--root-dir`` auto-loads every JSON file under
        ``mappings/`` at startup, and ``POST /__admin/reset`` reloads that
        baseline.  Stray file-loaded mappings (e.g. PUT/upload stubs with
        incorrect body filters, or 500/401 catch-all stubs) can interfere
        with unrelated tests.  We enumerate and delete individually because
        bulk ``DELETE /__admin/mappings`` fails with 500
        (NotWritableException) when multi-mapping files are present.

        After clearing, a low-priority logout stub is registered so that
        connection teardown always succeeds even if a test does not
        explicitly add a logout mapping.
        """
        base = self.http_url()
        # Reset scenarios and request journal first.
        resp = requests.post(f"{base}/__admin/reset", timeout=5)
        if resp.status_code not in (200, 201):
            raise RuntimeError(f"Failed to reset WireMock: {resp.status_code} {resp.text}")
        # Enumerate all stubs and delete one by one to remove file-loaded
        # mappings that __admin/reset just reloaded.
        listing = requests.get(f"{base}/__admin/mappings", timeout=5)
        if listing.status_code != 200:
            raise RuntimeError(f"Failed to list WireMock mappings: {listing.status_code} {listing.text}")
        for mapping in listing.json().get("mappings", []):
            mapping_id = mapping.get("id") or mapping.get("uuid")
            if mapping_id:
                requests.delete(f"{base}/__admin/mappings/{mapping_id}", timeout=5)
        # Register low-priority baseline stubs so connection teardown always
        # succeeds without tests needing to add them explicitly.
        resp = requests.post(
            f"{base}/__admin/mappings",
            json={
                "priority": 999,
                "request": {
                    "urlPath": "/session",
                    "method": "POST",
                    "queryParameters": {"delete": {"equalTo": "true"}},
                },
                "response": {
                    "status": 200,
                    "jsonBody": {"success": True},
                    "headers": {"Content-Type": "application/json"},
                },
            },
            timeout=5,
        )
        if resp.status_code not in (200, 201):
            raise RuntimeError(f"Failed to register logout baseline stub: {resp.status_code} {resp.text}")
        # Catch-all query response for driver-initiated queries (COMMIT, etc.)
        # that happen during connection teardown.
        resp = requests.post(
            f"{base}/__admin/mappings",
            json={
                "priority": 999,
                "request": {
                    "urlPathPattern": "/queries/v1/query-request.*",
                    "method": "POST",
                },
                "response": {
                    "status": 200,
                    "jsonBody": {
                        "success": True,
                        "data": {
                            "queryId": "baseline-catchall",
                            "queryResultFormat": "json",
                            "rowtype": [
                                {
                                    "name": "status",
                                    "type": "text",
                                    "nullable": True,
                                    "length": 16777216,
                                    "byteLength": 16777216,
                                    "precision": None,
                                    "scale": None,
                                }
                            ],
                            "rowset": [["Statement executed successfully."]],
                            "total": 1,
                            "returned": 1,
                            "parameters": [],
                        },
                        "code": None,
                        "message": None,
                    },
                    "headers": {"Content-Type": "application/json"},
                },
            },
            timeout=5,
        )
        if resp.status_code not in (200, 201):
            raise RuntimeError(f"Failed to register query baseline stub: {resp.status_code} {resp.text}")

    def get_all_requests(self) -> list:
        """Query admin API for all captured requests."""
        response = requests.get(f"{self.http_url()}/__admin/requests")
        return response.json().get("requests", [])

    def get_logout_requests(self) -> list:
        """Filter captured requests to logout requests (POST /session?delete=true)."""
        return [
            r
            for r in self.get_all_requests()
            if r.get("request", {}).get("method") == "POST"
            and "/session" in r.get("request", {}).get("url", "")
            and "delete=true" in r.get("request", {}).get("url", "")
        ]

    def get_requests(self, url_path_pattern: str) -> list[dict]:
        """Get all requests received by Wiremock matching a URL path pattern.

        Args:
            url_path_pattern: Regex pattern to match against request URL paths
                             (e.g., "/telemetry/send")

        Returns:
            List of request objects captured by Wiremock.
        """
        response = requests.post(
            f"{self.http_url()}/__admin/requests/find",
            json={"urlPathPattern": url_path_pattern},
            timeout=5,
        )
        if response.status_code != 200:
            raise RuntimeError(f"Failed to query requests: {response.status_code} {response.text}")
        return response.json().get("requests", [])

    def wait_for_requests(
        self, url_path_pattern: str, min_count: int = 1, timeout: float = 2.0, poll_interval: float = 0.1
    ) -> list[dict]:
        """Poll Wiremock until at least `min_count` requests matching the pattern arrive.

        Useful for asserting on requests that are sent asynchronously (e.g. telemetry).

        Args:
            url_path_pattern: Regex pattern to match against request URL paths.
            min_count: Minimum number of matching requests to wait for.
            timeout: Maximum time in seconds to wait before returning.
            poll_interval: Time in seconds between polls.

        Returns:
            List of matching request objects (may be fewer than min_count on timeout).
        """
        deadline = time.time() + timeout
        result: list[dict] = []
        while time.time() < deadline:
            result = self.get_requests(url_path_pattern)
            if len(result) >= min_count:
                return result
            time.sleep(poll_interval)
        return result

    def stop(self) -> None:
        """Stop the Wiremock process.

        This is automatically called when the object is garbage collected.
        """
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None
        if self._security_props_file is not None:
            try:
                os.unlink(self._security_props_file)
            except OSError:
                pass
            self._security_props_file = None

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - ensures cleanup."""
        self.stop()

    def __del__(self):
        """Destructor - ensures cleanup."""
        self.stop()

    @staticmethod
    def _find_free_port() -> int:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("", 0))
            s.listen(1)
            port = s.getsockname()[1]
        return port

    def _wait_for_health(self, max_retries: int = 60, sleep_seconds: float = 0.5) -> None:
        health_url = f"{self.http_url()}/__admin/health"
        last_error = None

        for _ in range(max_retries):
            time.sleep(sleep_seconds)

            if self.process.poll() is not None:
                stdout = self.process.stdout.read() if self.process.stdout else b""
                stderr = self.process.stderr.read() if self.process.stderr else b""
                raise RuntimeError(
                    f"Wiremock process died with exit code {self.process.returncode}\n"
                    f"stdout: {stdout.decode('utf-8', errors='ignore')}\n"
                    f"stderr: {stderr.decode('utf-8', errors='ignore')}"
                )

            try:
                response = requests.get(health_url, timeout=2)
                if response.status_code == 200:
                    text = response.text
                    if '"status"' in text and '"healthy"' in text:
                        return
            except requests.RequestException as e:
                last_error = str(e)

        raise RuntimeError(
            f"Wiremock did not become healthy after {max_retries * sleep_seconds} seconds. Last error: {last_error}"
        )

    def _wait_for_https_health(self, max_retries: int = 60, sleep_seconds: float = 0.5) -> None:
        health_url = f"{self.https_url()}/__admin/health"
        last_error = None

        for _ in range(max_retries):
            time.sleep(sleep_seconds)

            if self.process.poll() is not None:
                raise RuntimeError(f"Wiremock process died (exit {self.process.returncode}) before HTTPS became ready")

            try:
                response = requests.get(health_url, timeout=2, verify=False)
                if response.status_code == 200:
                    text = response.text
                    if '"status"' in text and '"healthy"' in text:
                        return
            except requests.RequestException as e:
                last_error = str(e)

        raise RuntimeError(
            f"Wiremock HTTPS did not become healthy after {max_retries * sleep_seconds} seconds. "
            f"Last error: {last_error}"
        )
