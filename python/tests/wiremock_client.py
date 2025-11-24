import json
import socket
import subprocess
import requests
from pathlib import Path
from typing import Dict, Optional

import time

from .utils import repo_root


WIREMOCK_VERSION = "3.13.2"
WIREMOCK_DIR = "tests/wiremock"
WIREMOCK_JAR_SUBDIR = "wiremock_standalone"
WIREMOCK_MAPPINGS_SUBDIR = "mappings"


class WiremockClient:

    def __init__(self):
        self.process: Optional[subprocess.Popen] = None
        self.http_port: Optional[int] = None
        self.host: str = "localhost"
        self.workspace_root: Optional[Path] = None
    
    def start(self) -> 'WiremockClient':
        """Start a new Wiremock instance.
        
        - Find a free port for HTTP
        - Start the Wiremock standalone JAR
        - Wait for Wiremock to be healthy
        """
        self.workspace_root = repo_root()
        wiremock_dir = self.workspace_root / WIREMOCK_DIR
        jar_path = wiremock_dir / WIREMOCK_JAR_SUBDIR / f"wiremock-standalone-{WIREMOCK_VERSION}.jar"
        
        if not jar_path.exists():
            raise FileNotFoundError(f"Wiremock JAR not found at: {jar_path}")
        
        self.http_port = self._find_free_port()
        
        self.process = subprocess.Popen(
            [
                "java",
                "-jar",
                str(jar_path),
                "--root-dir",
                str(wiremock_dir),
                "--enable-browser-proxying",  # work as forward proxy
                "--proxy-pass-through",
                "false",  # pass through only matched requests
                "--port",
                str(self.http_port),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        
        self._wait_for_health()
        return self
    
    def http_url(self) -> str:
        """Get the HTTP URL for connecting to this Wiremock instance.
        
        Returns:
            HTTP URL string (e.g., "http://localhost:12345")
        """
        return f"http://{self.host}:{self.http_port}"
    
    def add_mapping(self, mapping_path: str, placeholders: Optional[Dict[str, str]] = None) -> None:
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
            "{{REPO_ROOT}}": str(self.workspace_root),
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
                response = requests.post(admin_url, json=mapping)
                
                if response.status_code not in (200, 201):
                    raise RuntimeError(
                        f"Failed to add mapping: {response.status_code} {response.text}"
                    )
        else:
            # Single mapping - send the entire content as-is
            response = requests.post(
                admin_url,
                data=content,
                headers={"Content-Type": "application/json"}
            )
            
            if response.status_code not in (200, 201):
                raise RuntimeError(
                    f"Failed to add mapping: {response.status_code} {response.text}"
                )
    
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
                stdout = self.process.stdout.read() if self.process.stdout else b''
                stderr = self.process.stderr.read() if self.process.stderr else b''
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
            f"Wiremock did not become healthy after {max_retries * sleep_seconds} seconds. "
            f"Last error: {last_error}"
        )

