//! DNS resolution for WASM via host functions.
//!
//! Since std::net::ToSocketAddrs doesn't work in WASM (returns "operation not supported"),
//! we use a custom host function to resolve DNS.

use std::net::SocketAddr;
#[cfg(all(feature = "wasm", not(feature = "native")))]
use std::net::{IpAddr, Ipv4Addr};

#[cfg(all(feature = "wasm", not(feature = "native")))]
#[link(wasm_import_module = "dns")]
unsafe extern "C" {
    /// Resolve a hostname to an IPv4 address.
    /// Returns 0 on success, 1 on error.
    /// On success, result_ptr will contain: [ok: u8, ip0: u8, ip1: u8, ip2: u8, ip3: u8]
    fn resolve(hostname_ptr: *const u8, hostname_len: u32, result_ptr: *mut u8) -> u32;
}

/// Resolve a hostname to a socket address using the host's DNS resolver.
#[cfg(all(feature = "wasm", not(feature = "native")))]
pub fn resolve_host(hostname: &str, port: u16) -> std::io::Result<SocketAddr> {
    let mut result = [0u8; 5];

    let ret = unsafe {
        resolve(
            hostname.as_ptr(),
            hostname.len() as u32,
            result.as_mut_ptr(),
        )
    };

    if ret != 0 || result[0] != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("DNS resolution failed for {}", hostname),
        ));
    }

    let ip = Ipv4Addr::new(result[1], result[2], result[3], result[4]);
    Ok(SocketAddr::new(IpAddr::V4(ip), port))
}

/// Native DNS resolution (uses std::net).
#[cfg(feature = "native")]
pub fn resolve_host(hostname: &str, port: u16) -> std::io::Result<SocketAddr> {
    use std::net::ToSocketAddrs;

    let addr_str = format!("{}:{}", hostname, port);
    addr_str.to_socket_addrs()?.next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No addresses found for {}", hostname),
        )
    })
}
