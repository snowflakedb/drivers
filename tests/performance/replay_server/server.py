#!/usr/bin/env python3
"""Lightweight HTTPS MITM proxy for performance testing.

Supports two modes:
  record  — Forward requests to the real backend, save request/response pairs
            as mapping files on disk (same format as the replay phase reads).
  replay  — Load mapping files into memory and serve recorded responses with
            minimal overhead (zero disk I/O per request).

Architecture:
    1. Generates a CA cert for MITM TLS on-the-fly
    2. Listens as an HTTP forward proxy (handles CONNECT tunneling)
    3. In record mode: forwards to real backend, saves mappings to disk
    4. In replay mode: matches requests by URL pattern, serves from memory
"""

import argparse
import base64
import datetime
import http.client
import json
import os
import re
import signal
import socket
import ssl
import sys
import tempfile
import threading
import time
import uuid
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID

TELEMETRY_PATTERN = re.compile(r'/telemetry/send')
TELEMETRY_RESPONSE = json.dumps({
    "data": "Log Received", "code": None,
    "message": None, "success": True,
}).encode()


class MappingEntry:
    """A single URL->response mapping loaded from a JSON file."""
    __slots__ = ('priority', 'method', 'pattern', 'status', 'headers', 'body')

    def __init__(self, priority, method, pattern, status, headers, body):
        self.priority = priority
        self.method = method
        self.pattern = pattern
        self.status = status
        self.headers = headers
        self.body = body


class ProxyServer:
    """HTTPS MITM proxy that can record or replay HTTP traffic."""

    def __init__(self, mappings_dir, port=8080, mode='replay', stats_suffix=None):
        self.mappings_dir = Path(mappings_dir)
        self.port = port
        self.mode = mode
        self.stats_suffix = stats_suffix

        # Replay state
        self.mappings: list[MappingEntry] = []

        # Recording state
        self._mapping_counter = 0
        self._mapping_lock = threading.Lock()
        self._mappings_path_created = False

        # Response time tracking (milliseconds, float precision)
        self.response_times: list[float] = []
        self.unmatched_requests: list[str] = []
        self.times_lock = threading.Lock()

        # TLS
        self.ca_key = None
        self.ca_cert = None
        self.ssl_contexts: dict[str, ssl.SSLContext] = {}
        self.certs_dir = tempfile.mkdtemp(prefix='proxy_certs_')

        self._running = True

    # ── Mapping Loading (replay mode) ────────────────────────────────

    def load_mappings(self):
        """Load all mapping JSON files into memory."""
        mappings_path = self.mappings_dir / "mappings"
        if not mappings_path.exists():
            raise RuntimeError(f"Mappings directory not found: {mappings_path}")

        files = list(mappings_path.glob("*.json"))
        if not files:
            raise RuntimeError(f"No mapping files in: {mappings_path}")

        total_body_bytes = 0
        for f in files:
            with open(f) as fh:
                data = json.load(fh)

            request = data.get("request", {})
            response = data.get("response", {})
            priority = data.get("priority", 100)

            url_pattern = request.get("urlPattern")
            url_path = request.get("urlPath")
            url_exact = request.get("url")

            if url_pattern:
                pattern = re.compile(url_pattern)
            elif url_path:
                pattern = re.compile(re.escape(url_path) + r"($|\?.*)")
            elif url_exact:
                pattern = re.compile(re.escape(url_exact) + "$")
            else:
                continue

            method = request.get("method", "ANY")
            status = response.get("status", 200)
            headers = response.get("headers", {})

            base64_body = response.get("base64Body")
            body_str = response.get("body", "")

            if base64_body:
                body = base64.b64decode(base64_body)
            elif body_str:
                body = body_str.encode("utf-8")
            else:
                body = b""

            total_body_bytes += len(body)

            self.mappings.append(MappingEntry(
                priority=priority,
                method=method,
                pattern=pattern,
                status=status,
                headers=headers,
                body=body,
            ))

        # Built-in telemetry mock
        self.mappings.append(MappingEntry(
            priority=0,
            method="POST",
            pattern=TELEMETRY_PATTERN,
            status=200,
            headers={"Content-Type": "application/json"},
            body=TELEMETRY_RESPONSE,
        ))

        self.mappings.sort(key=lambda m: m.priority)
        print(f"Loaded {len(self.mappings)} mappings "
              f"({total_body_bytes / 1024 / 1024:.1f} MB in memory)")

    # ── TLS / Certificate Generation ─────────────────────────────────

    def generate_ca(self):
        """Generate CA key and certificate for MITM TLS."""
        self.ca_key = rsa.generate_private_key(
            public_exponent=65537, key_size=2048)

        name = x509.Name([
            x509.NameAttribute(NameOID.COMMON_NAME, "Proxy Server CA"),
        ])
        now = datetime.datetime.now(datetime.timezone.utc)

        self.ca_cert = (
            x509.CertificateBuilder()
            .subject_name(name)
            .issuer_name(name)
            .public_key(self.ca_key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(now)
            .not_valid_after(now + datetime.timedelta(days=3650))
            .add_extension(
                x509.BasicConstraints(ca=True, path_length=None), critical=True)
            .add_extension(
                x509.KeyUsage(
                    digital_signature=False, key_encipherment=False,
                    content_commitment=False, data_encipherment=False,
                    key_agreement=False, key_cert_sign=True, crl_sign=True,
                    encipher_only=False, decipher_only=False,
                ), critical=True)
            .sign(self.ca_key, hashes.SHA256())
        )
        print("Generated CA certificate")

    def export_ca_cert(self, target_path):
        """Export CA certificate in PEM format."""
        with open(target_path, 'wb') as f:
            f.write(self.ca_cert.public_bytes(serialization.Encoding.PEM))
        print(f"Exported CA cert to {target_path}")

    def get_ssl_context(self, hostname):
        """Get or create a server-side SSL context for a hostname (cached)."""
        if hostname in self.ssl_contexts:
            return self.ssl_contexts[hostname]

        key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
        now = datetime.datetime.now(datetime.timezone.utc)

        cert = (
            x509.CertificateBuilder()
            .subject_name(x509.Name([
                x509.NameAttribute(NameOID.COMMON_NAME, hostname),
            ]))
            .issuer_name(self.ca_cert.subject)
            .public_key(key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(now)
            .not_valid_after(now + datetime.timedelta(days=365))
            .add_extension(
                x509.SubjectAlternativeName([x509.DNSName(hostname)]),
                critical=False,
            )
            .sign(self.ca_key, hashes.SHA256())
        )

        cert_path = os.path.join(self.certs_dir, f"{hostname}.crt")
        key_path = os.path.join(self.certs_dir, f"{hostname}.key")

        with open(cert_path, 'wb') as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))
        with open(key_path, 'wb') as f:
            f.write(key.private_bytes(
                serialization.Encoding.PEM,
                serialization.PrivateFormat.TraditionalOpenSSL,
                serialization.NoEncryption(),
            ))

        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert_path, key_path)
        self.ssl_contexts[hostname] = ctx
        print(f"Generated TLS cert for {hostname}")
        return ctx

    # ── Request Matching (replay mode) ───────────────────────────────

    def match_request(self, method, path):
        """Find best matching mapping (sorted by priority, first match wins)."""
        for m in self.mappings:
            if m.method != "ANY" and m.method != method:
                continue
            if m.pattern.search(path):
                return m
        return None

    # ── HTTP I/O ─────────────────────────────────────────────────────

    @staticmethod
    def _send_http_response(sock, status, headers, body):
        """Serialize and send an HTTP/1.1 response."""
        status_text = {
            200: 'OK', 404: 'Not Found', 500: 'Internal Server Error',
            302: 'Found', 400: 'Bad Request', 403: 'Forbidden',
        }.get(status, 'Unknown')

        lines = [f"HTTP/1.1 {status} {status_text}"]

        has_cl = any(k.lower() == 'content-length' for k in headers)
        if not has_cl:
            lines.append(f"Content-Length: {len(body)}")

        lines.append("Connection: keep-alive")

        for key, value in headers.items():
            if key.lower() == 'transfer-encoding':
                continue
            lines.append(f"{key}: {value}")

        header_bytes = ("\r\n".join(lines) + "\r\n\r\n").encode('utf-8')
        sock.sendall(header_bytes + body)

    @staticmethod
    def _recv_until(sock, delimiter, max_size=65536):
        """Receive data until delimiter is found."""
        data = b""
        while delimiter not in data and len(data) < max_size:
            chunk = sock.recv(4096)
            if not chunk:
                return None
            data += chunk
        return data

    @staticmethod
    def _parse_request(ssl_sock, buf):
        """Read one HTTP request from the TLS socket. Returns (method, path, headers_dict, body, remaining_buf) or None."""
        while b'\r\n\r\n' not in buf:
            chunk = ssl_sock.recv(65536)
            if not chunk:
                return None
            buf += chunk

        header_end = buf.index(b'\r\n\r\n') + 4
        header_data = buf[:header_end]
        buf = buf[header_end:]

        lines = header_data.decode('utf-8', errors='replace').split('\r\n')
        parts = lines[0].split(' ', 2)
        if len(parts) < 2:
            return None

        method, path = parts[0], parts[1]

        hdrs = {}
        for line in lines[1:]:
            if ':' in line:
                k, v = line.split(':', 1)
                hdrs[k.strip().lower()] = v.strip()

        content_length = int(hdrs.get('content-length', '0'))
        request_body = b""
        if content_length > 0:
            while len(buf) < content_length:
                chunk = ssl_sock.recv(65536)
                if not chunk:
                    return None
                buf += chunk
            request_body = buf[:content_length]
            buf = buf[content_length:]

        return method, path, hdrs, request_body, buf

    # ── Replay Tunnel ────────────────────────────────────────────────

    def _handle_tunnel_replay(self, ssl_sock):
        """Handle multiple HTTP requests over a single TLS tunnel (replay mode)."""
        buf = b""
        while self._running:
            try:
                parsed = self._parse_request(ssl_sock, buf)
                if parsed is None:
                    return
                method, path, hdrs, _, buf = parsed

                start = time.monotonic()
                mapping = self.match_request(method, path)

                if mapping:
                    self._send_http_response(
                        ssl_sock, mapping.status, mapping.headers, mapping.body)
                else:
                    self._send_http_response(
                        ssl_sock, 404, {},
                        f"No mapping for {method} {path}".encode())
                    with self.times_lock:
                        self.unmatched_requests.append(f"{method} {path}")
                    print(f"WARNING: No mapping for {method} {path}")

                elapsed_ms = (time.monotonic() - start) * 1000
                with self.times_lock:
                    self.response_times.append(elapsed_ms)

                if hdrs.get('connection', '').lower() == 'close':
                    return

            except (ConnectionResetError, BrokenPipeError,
                    ssl.SSLError, OSError):
                return

    # ── Recording Tunnel ─────────────────────────────────────────────

    def _handle_tunnel_record(self, ssl_sock, hostname, port=443):
        """Forward requests to real backend and save as mapping files."""
        backend = http.client.HTTPSConnection(hostname, port, timeout=120)

        buf = b""
        try:
            while self._running:
                try:
                    parsed = self._parse_request(ssl_sock, buf)
                    if parsed is None:
                        return
                    method, path, hdrs, request_body, buf = parsed

                    # Intercept telemetry — don't forward to real backend
                    if TELEMETRY_PATTERN.search(path):
                        self._send_http_response(
                            ssl_sock, 200,
                            {"Content-Type": "application/json"},
                            TELEMETRY_RESPONSE)
                        continue

                    # Forward to real backend
                    fwd_headers = {k: v for k, v in hdrs.items()
                                   if k not in ('transfer-encoding', 'connection',
                                                'proxy-connection', 'keep-alive')}
                    fwd_headers['host'] = hostname

                    try:
                        backend.request(method, path, body=request_body or None,
                                        headers=fwd_headers)
                        resp = backend.getresponse()
                    except (http.client.RemoteDisconnected, ConnectionError,
                            OSError):
                        # Reconnect once on connection reset
                        backend = http.client.HTTPSConnection(
                            hostname, port, timeout=120)
                        backend.request(method, path, body=request_body or None,
                                        headers=fwd_headers)
                        resp = backend.getresponse()

                    resp_body = resp.read()
                    resp_headers = {k: v for k, v in resp.getheaders()}

                    # Save mapping to disk
                    self._save_mapping(method, path, resp.status,
                                       resp_headers, resp_body, request_body)

                    # Relay response back to client
                    self._send_http_response(
                        ssl_sock, resp.status, resp_headers, resp_body)

                    if hdrs.get('connection', '').lower() == 'close':
                        return

                except (ConnectionResetError, BrokenPipeError,
                        ssl.SSLError, OSError):
                    return
        finally:
            try:
                backend.close()
            except Exception:
                pass

    def _save_mapping(self, method, path, status, headers, body, request_body):
        """Save a recorded request/response pair as a mapping JSON file."""
        mappings_path = self.mappings_dir / "mappings"
        if not self._mappings_path_created:
            mappings_path.mkdir(parents=True, exist_ok=True)
            self._mappings_path_created = True

        mapping_id = str(uuid.uuid4())
        safe_name = path.split('?')[0].strip('/').replace('/', '_')
        if len(safe_name) > 80:
            safe_name = safe_name[:80]

        content_type = headers.get('Content-Type', headers.get('content-type', ''))
        is_text = ('json' in content_type or 'text' in content_type
                   or 'xml' in content_type)

        response_data = {
            "status": status,
            "headers": headers,
        }

        if is_text:
            try:
                response_data["body"] = body.decode('utf-8')
            except UnicodeDecodeError:
                response_data["base64Body"] = base64.b64encode(body).decode('ascii')
        elif body:
            response_data["base64Body"] = base64.b64encode(body).decode('ascii')

        request_data = {
            "url": path,
            "method": method,
        }

        if request_body and method in ('POST', 'PUT', 'PATCH'):
            try:
                body_text = request_body.decode('utf-8')
                request_data["bodyPatterns"] = [{"equalTo": body_text}]
            except UnicodeDecodeError:
                pass

        mapping = {
            "request": request_data,
            "response": response_data,
            "priority": 100,
        }

        filename = f"{safe_name}-{mapping_id}.json"
        filepath = mappings_path / filename
        with open(filepath, 'w') as f:
            json.dump(mapping, f, indent=2)

    # ── Connection Handling ───────────────────────────────────────────

    def _handle_client(self, client_sock):
        """Handle a single client connection (CONNECT tunnel or admin request)."""
        try:
            data = self._recv_until(client_sock, b'\r\n\r\n')
            if not data:
                return

            first_line = data.split(b'\r\n')[0].decode('utf-8', errors='replace')
            parts = first_line.split()

            if len(parts) < 3:
                return

            method, target = parts[0], parts[1]

            if method == "CONNECT":
                host_port = target.split(':')
                hostname = host_port[0]
                port = int(host_port[1]) if len(host_port) > 1 else 443

                client_sock.sendall(
                    b'HTTP/1.1 200 Connection Established\r\n\r\n')

                ssl_ctx = self.get_ssl_context(hostname)
                ssl_sock = ssl_ctx.wrap_socket(client_sock, server_side=True)

                try:
                    if self.mode == 'record':
                        self._handle_tunnel_record(ssl_sock, hostname, port)
                    else:
                        self._handle_tunnel_replay(ssl_sock)
                finally:
                    try:
                        ssl_sock.close()
                    except OSError:
                        pass

            elif target == "/__perf/flush-stats":
                self._write_stats()
                client_sock.sendall(
                    b'HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nflushed')

            elif target == "/__admin/health":
                client_sock.sendall(
                    b'HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok')

            else:
                msg = b"Only CONNECT tunneling is supported"
                client_sock.sendall(
                    f"HTTP/1.1 400 Bad Request\r\n"
                    f"Content-Length: {len(msg)}\r\n\r\n".encode() + msg)

        except Exception as e:
            print(f"Client error: {e}", file=sys.stderr)
        finally:
            try:
                client_sock.close()
            except OSError:
                pass

    # ── Stats ────────────────────────────────────────────────────────

    def _write_stats(self):
        """Write response time stats to JSON file on the mapped volume."""
        suffix = f"-{self.stats_suffix}" if self.stats_suffix else ""
        stats_file = self.mappings_dir / f"response-time-stats{suffix}.json"

        with self.times_lock:
            times_copy = list(self.response_times)
            unmatched_copy = list(self.unmatched_requests)

        stats = {
            "total_requests": len(times_copy),
            "response_times": times_copy,
            "unmatched_requests": len(unmatched_copy),
            "unmatched_details": unmatched_copy[:20],
        }

        with open(stats_file, 'w') as f:
            json.dump(stats, f)

        print(f"Wrote stats: {len(times_copy)} requests -> {stats_file.name}")

    # ── Main Loop ────────────────────────────────────────────────────

    def run(self):
        """Start the proxy server."""
        if self.mode == 'replay':
            self.load_mappings()

        self.generate_ca()

        ca_cert_path = self.mappings_dir / "replay-ca.crt"
        self.export_ca_cert(ca_cert_path)

        def _shutdown(signum, _frame):
            print(f"Received signal {signum}, shutting down...")
            self._running = False

        signal.signal(signal.SIGTERM, _shutdown)
        signal.signal(signal.SIGINT, _shutdown)

        server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server_sock.bind(('0.0.0.0', self.port))
        server_sock.listen(128)
        server_sock.settimeout(1.0)

        print(f"Proxy server ({self.mode} mode) listening on :{self.port}")

        while self._running:
            try:
                client_sock, _ = server_sock.accept()
                t = threading.Thread(
                    target=self._handle_client, args=(client_sock,), daemon=True)
                t.start()
            except socket.timeout:
                continue
            except Exception as e:
                if self._running:
                    print(f"Accept error: {e}", file=sys.stderr)

        server_sock.close()
        self._write_stats()
        print("Server stopped.")


def main():
    parser = argparse.ArgumentParser(
        description="Lightweight HTTPS recording/replay proxy")
    parser.add_argument('--mappings-dir', required=True,
                        help="Root directory (contains mappings/ subfolder)")
    parser.add_argument('--port', type=int, default=8080)
    parser.add_argument('--mode', choices=['record', 'replay'], default='replay',
                        help="Operating mode: record (forward+save) or replay (serve from disk)")
    parser.add_argument('--stats-suffix', default=None,
                        help="Suffix for stats file (e.g. 'universal')")
    args = parser.parse_args()

    server = ProxyServer(args.mappings_dir, args.port, args.mode, args.stats_suffix)
    server.run()


if __name__ == '__main__':
    main()
