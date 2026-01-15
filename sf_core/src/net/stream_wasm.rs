//! WASM TCP stream implementation using wstd (wasi-sockets).
//!
//! wstd uses async-fn based traits rather than poll-based traits.
//! This module provides a TCP connector and stream wrapper that
//! exposes async methods for use with wstd's async runtime.
//!
//! DNS resolution is handled by a custom host function since
//! std::net::ToSocketAddrs doesn't work in WASM.

use super::dns;
use super::error::TcpConnectorError;
use std::io;
use wstd::io::{AsyncRead as WstdAsyncRead, AsyncWrite as WstdAsyncWrite};
use wstd::net::TcpStream as WstdTcpStream;

/// WASM TCP stream wrapper around wstd::net::TcpStream.
///
/// This provides async methods that work with wstd's async runtime.
/// Note: This does NOT implement the poll-based traits since wstd
/// uses async-fn based I/O.
pub struct WasmTcpStream {
    inner: WstdTcpStream,
}

impl WasmTcpStream {
    /// Create a new WASM TCP stream from a wstd stream.
    pub fn new(inner: WstdTcpStream) -> Self {
        Self { inner }
    }

    /// Get a reference to the inner wstd stream.
    pub fn inner(&self) -> &WstdTcpStream {
        &self.inner
    }

    /// Get a mutable reference to the inner wstd stream.
    pub fn inner_mut(&mut self) -> &mut WstdTcpStream {
        &mut self.inner
    }

    /// Consume this wrapper and return the inner wstd stream.
    pub fn into_inner(self) -> WstdTcpStream {
        self.inner
    }

    /// Read data from the stream.
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).await
    }

    /// Write data to the stream.
    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).await
    }

    /// Flush the stream.
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().await
    }

    /// Write all bytes to the stream.
    pub async fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf).await?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "write returned 0"));
            }
            buf = &buf[n..];
        }
        Ok(())
    }

    /// Read exact number of bytes.
    pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let n = self.read(buf).await?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected EOF",
                ));
            }
            buf = &mut buf[n..];
        }
        Ok(())
    }
}

/// WASM TCP connector using wstd (wasi-sockets).
#[derive(Clone, Default)]
pub struct WasmTcpConnector;

impl WasmTcpConnector {
    /// Create a new WASM TCP connector.
    pub fn new() -> Self {
        Self
    }

    /// Connect to the specified host and port.
    ///
    /// This resolves DNS using the host's resolver (since std::net doesn't work in WASM),
    /// then connects to the resolved IP address.
    pub async fn connect(&self, host: &str, port: u16) -> Result<WasmTcpStream, TcpConnectorError> {
        // Resolve DNS using host function
        let socket_addr =
            dns::resolve_host(host, port).map_err(|e| TcpConnectorError::ConnectionFailed {
                host: host.to_string(),
                port,
                message: format!("DNS resolution failed: {}", e),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Connect using the resolved address
        let stream = WstdTcpStream::connect_addr(socket_addr)
            .await
            .map_err(|e| TcpConnectorError::ConnectionFailed {
                host: host.to_string(),
                port,
                message: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        Ok(WasmTcpStream::new(stream))
    }
}
