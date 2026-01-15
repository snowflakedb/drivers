//! WASM HTTP client implementation with TLS support.
//!
//! This client uses raw TCP via wasi:sockets and performs TLS using
//! rustls with the RustCrypto backend (pure Rust, WASM-compatible).

use super::client::{
    Headers, HttpClient, HttpClientConfig, HttpClientError, HttpRequest, HttpResponse, Method,
    StatusCode,
};
use crate::net::{WasmTcpConnector, WasmTcpStream};
use rustls;
use std::sync::Arc;

/// WASM HTTP client with TLS support.
///
/// Uses raw TCP via wasi:sockets and rustls with RustCrypto for TLS.
pub struct WasmHttpClient {
    tcp_connector: WasmTcpConnector,
    tls_config: Arc<rustls::ClientConfig>,
    _config: HttpClientConfig,
}

impl WasmHttpClient {
    /// Create a new WASM HTTP client with default configuration.
    pub fn new() -> Result<Self, HttpClientError> {
        Self::with_config(HttpClientConfig::new())
    }

    /// Create a new WASM HTTP client with custom configuration.
    pub fn with_config(config: HttpClientConfig) -> Result<Self, HttpClientError> {
        // Build TLS config with RustCrypto provider
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // Force TLS 1.2 only - rustls-rustcrypto doesn't fully support TLS 1.3
        let tls_config =
            rustls::ClientConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
                .with_protocol_versions(&[&rustls::version::TLS12])
                .map_err(|e| HttpClientError::Tls {
                    message: format!("Failed to set TLS protocol versions: {e}"),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?
                .with_root_certificates(root_store)
                .with_no_client_auth();

        Ok(Self {
            tcp_connector: WasmTcpConnector::new(),
            tls_config: Arc::new(tls_config),
            _config: config,
        })
    }

    async fn send_http_request(
        &self,
        request: &HttpRequest,
        host: &str,
        port: u16,
        path: &str,
        use_tls: bool,
    ) -> Result<HttpResponse, HttpClientError> {
        // Connect via TCP - the Go host shim handles TLS transparently for port 443
        let stream = self.tcp_connector.connect(host, port).await.map_err(|e| {
            HttpClientError::Connection {
                message: format!("TCP connection failed: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            }
        })?;

        if use_tls {
            self.send_https_request(request, host, path, stream).await
        } else {
            self.send_plain_http_request(request, host, path, stream)
                .await
        }
    }

    async fn send_https_request(
        &self,
        request: &HttpRequest,
        host: &str,
        path: &str,
        tcp_stream: WasmTcpStream,
    ) -> Result<HttpResponse, HttpClientError> {
        use rustls::pki_types::ServerName;

        // Create TLS connection
        let server_name =
            ServerName::try_from(host.to_string()).map_err(|e| HttpClientError::Tls {
                message: format!("Invalid server name '{host}': {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let tls_conn = rustls::ClientConnection::new(self.tls_config.clone(), server_name)
            .map_err(|e| HttpClientError::Tls {
                message: format!("Failed to create TLS connection: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Create a TLS stream wrapper
        let mut tls_stream = TlsStream::new(tcp_stream, Box::new(tls_conn));

        // Perform TLS handshake
        tls_stream
            .handshake()
            .await
            .map_err(|e| HttpClientError::Tls {
                message: format!("TLS handshake failed: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Send HTTP request over TLS
        self.send_request_to_stream(&mut tls_stream, request, host, path)
            .await
    }

    async fn send_plain_http_request(
        &self,
        request: &HttpRequest,
        host: &str,
        path: &str,
        mut stream: WasmTcpStream,
    ) -> Result<HttpResponse, HttpClientError> {
        self.send_request_to_stream(&mut stream, request, host, path)
            .await
    }

    async fn send_request_to_stream<S: AsyncReadWrite>(
        &self,
        stream: &mut S,
        request: &HttpRequest,
        host: &str,
        path: &str,
    ) -> Result<HttpResponse, HttpClientError> {
        // Build HTTP request
        let method = match request.method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Patch => "PATCH",
            Method::Options => "OPTIONS",
        };

        // Build path with query parameters
        let full_path = if request.query_params.is_empty() {
            path.to_string()
        } else {
            let query_string: String = request
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", path, query_string)
        };

        let mut request_str = format!("{} {} HTTP/1.1\r\n", method, full_path);
        request_str.push_str(&format!("Host: {}\r\n", host));

        // Add Content-Length first (like curl does)
        if let Some(ref body) = request.body {
            request_str.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }

        // Add other headers in specific order
        if let Some(ct) = request.headers.get("Content-Type") {
            request_str.push_str(&format!("Content-Type: {}\r\n", ct));
        }
        if let Some(accept) = request.headers.get("Accept") {
            request_str.push_str(&format!("Accept: {}\r\n", accept));
        }
        if let Some(auth) = request.headers.get("Authorization") {
            request_str.push_str(&format!("Authorization: {}\r\n", auth));
        }
        if let Some(ua) = request.headers.get("User-Agent") {
            request_str.push_str(&format!("User-Agent: {}\r\n", ua));
        }

        // Add any remaining headers
        for (key, value) in &request.headers {
            if !["Content-Type", "Accept", "Authorization", "User-Agent"].contains(&key.as_str()) {
                request_str.push_str(&format!("{}: {}\r\n", key, value));
            }
        }

        // Connection header last
        request_str.push_str("Connection: close\r\n");

        request_str.push_str("\r\n");

        // Write request
        stream
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| HttpClientError::Io {
                message: format!("Failed to write request: {}", e),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        if let Some(ref body) = request.body {
            stream
                .write_all(body)
                .await
                .map_err(|e| HttpClientError::Io {
                    message: format!("Failed to write body: {}", e),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;
        }

        // Read response
        let response_data = read_response(stream)
            .await
            .map_err(|e| HttpClientError::Io {
                message: format!("Failed to read response: {}", e),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        parse_http_response(&response_data)
    }
}

impl Default for WasmHttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default WASM HTTP client")
    }
}

impl HttpClient for WasmHttpClient {
    async fn request(&self, request: HttpRequest) -> Result<HttpResponse, HttpClientError> {
        // Parse URL
        let url = url::Url::parse(&request.url).map_err(|e| HttpClientError::RequestBuild {
            message: format!("Invalid URL: {}", e),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        let scheme = url.scheme();
        let host = url
            .host_str()
            .ok_or_else(|| HttpClientError::RequestBuild {
                message: "URL missing host".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let use_tls = scheme == "https";
        let port = url.port().unwrap_or(if use_tls { 443 } else { 80 });
        let path = if url.query().is_some() {
            format!("{}?{}", url.path(), url.query().unwrap())
        } else {
            url.path().to_string()
        };

        self.send_http_request(&request, host, port, &path, use_tls)
            .await
    }
}

/// Trait for async read/write operations (implemented by both TcpStream and TlsStream)
trait AsyncReadWrite {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

impl AsyncReadWrite for WasmTcpStream {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        WasmTcpStream::read(self, buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        WasmTcpStream::write_all(self, buf).await
    }
}

/// TLS stream wrapper using rustls
struct TlsStream {
    tcp: WasmTcpStream,
    tls: Box<rustls::ClientConnection>,
    /// Unconsumed bytes from previous TCP reads
    pending_read: Vec<u8>,
}

impl TlsStream {
    fn new(tcp: WasmTcpStream, tls: Box<rustls::ClientConnection>) -> Self {
        Self {
            tcp,
            tls,
            pending_read: Vec::new(),
        }
    }

    async fn handshake(&mut self) -> std::io::Result<()> {
        while self.tls.is_handshaking() {
            self.do_tls_io().await?;
        }
        Ok(())
    }

    async fn do_tls_io(&mut self) -> std::io::Result<()> {
        // Write any pending TLS data to TCP
        while self.tls.wants_write() {
            let mut buf = Vec::new();
            self.tls.write_tls(&mut buf)?;
            if !buf.is_empty() {
                self.tcp.write_all(&buf).await?;
            }
        }

        // Read TLS data from TCP if needed
        if self.tls.wants_read() {
            // Read from TCP into our pending buffer
            let mut buf = vec![0u8; 16384];
            let n = self.tcp.read(&mut buf).await?;

            if n == 0 && self.pending_read.is_empty() {
                return Ok(());
            }

            // Append to pending buffer
            if n > 0 {
                self.pending_read.extend_from_slice(&buf[..n]);
            }

            // Feed all pending data to rustls in chunks
            // rustls has an internal ~4KB buffer limit, so we loop until all data is consumed
            while !self.pending_read.is_empty() && self.tls.wants_read() {
                let pending = std::mem::take(&mut self.pending_read);
                let mut slice = &pending[..];
                let read_result = self.tls.read_tls(&mut slice);

                // Save unconsumed bytes for next iteration
                let bytes_consumed = pending.len() - slice.len();
                if bytes_consumed < pending.len() {
                    self.pending_read = pending[bytes_consumed..].to_vec();
                }

                match read_result {
                    Ok(0) => break,
                    Ok(_) => {
                        self.tls
                            .process_new_packets()
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }
}

impl AsyncReadWrite for TlsStream {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            // Try to read decrypted data from rustls
            use std::io::Read;
            match self.tls.reader().read(buf) {
                Ok(n) => return Ok(n),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        // Need more TLS data
                        self.do_tls_io().await?;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        // Write plaintext to rustls
        self.tls.writer().write_all(buf)?;
        // Flush TLS data to TCP
        self.do_tls_io().await
    }
}

/// Read all data from the stream until connection close.
async fn read_response<S: AsyncReadWrite>(stream: &mut S) -> std::io::Result<Vec<u8>> {
    let mut response_data = Vec::new();
    let mut buf = [0u8; 8192];

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => response_data.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        }
    }

    Ok(response_data)
}

fn parse_http_response(data: &[u8]) -> Result<HttpResponse, HttpClientError> {
    // Find header/body boundary
    let header_end = find_header_end(data).ok_or_else(|| HttpClientError::ResponseParse {
        message: "Could not find header boundary".to_string(),
        location: snafu::Location::new(file!(), line!(), column!()),
    })?;

    let header_bytes = &data[..header_end];
    let body = data[header_end + 4..].to_vec(); // +4 to skip \r\n\r\n

    let header_str = String::from_utf8_lossy(header_bytes);
    let mut lines = header_str.lines();

    // Parse status line
    let status_line = lines.next().ok_or_else(|| HttpClientError::ResponseParse {
        message: "Empty response".to_string(),
        location: snafu::Location::new(file!(), line!(), column!()),
    })?;

    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(HttpClientError::ResponseParse {
            message: format!("Invalid status line: {}", status_line),
            location: snafu::Location::new(file!(), line!(), column!()),
        });
    }

    let status_code: u16 = parts[1]
        .parse()
        .map_err(|_| HttpClientError::ResponseParse {
            message: format!("Invalid status code: {}", parts[1]),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

    // Parse headers
    let mut headers: Headers = std::collections::HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    Ok(HttpResponse {
        status: StatusCode(status_code),
        headers,
        body,
    })
}

/// Find the position of the header/body boundary (\r\n\r\n).
fn find_header_end(data: &[u8]) -> Option<usize> {
    (0..data.len().saturating_sub(3)).find(|&i| &data[i..i + 4] == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_response() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello";
        let parsed = parse_http_response(response).unwrap();
        assert_eq!(parsed.status.0, 200);
        assert!(parsed.headers.contains_key("Content-Type"));
        assert_eq!(parsed.body, b"Hello");
    }

    #[test]
    fn test_find_header_end() {
        // "HTTP/1.1 200 OK" = 15 bytes, then \r\n\r\n starts at index 15
        let data = b"HTTP/1.1 200 OK\r\n\r\nbody";
        assert_eq!(find_header_end(data), Some(15));
    }
}
