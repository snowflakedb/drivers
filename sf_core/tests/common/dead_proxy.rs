//! Shared negative-control fixture: point some proxy-routed operation at a
//! dead loopback port (bound, then immediately dropped, so nothing answers)
//! and assert it fails. Covers both shapes seen across the proxy test suites:
//! the live-mitmdump e2e tests, where `connect()` itself is the operation
//! that must fail through the dead port, and the hermetic mock-GS
//! integration tests, where `connect()` succeeds (login bypasses the dead
//! port via `no_proxy`) and a subsequent transfer call is what must fail.

/// Binds an ephemeral loopback port and immediately drops the listener, so
/// nothing answers there. Any proxy-routed operation pointed at this port
/// must fail purely because nothing is listening — not because of some
/// unrelated config or auth problem.
pub fn dead_loopback_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Runs `attempt` against a freshly minted dead loopback port and asserts it
/// fails. `attempt` receives the dead port and performs whatever
/// proxy-routed operation is under test — e.g. building a client pointed at
/// the port and calling `.connect()`, or connecting (which may itself bypass
/// the dead port, e.g. via `no_proxy`) and then attempting a transfer through
/// it — returning that operation's `Result`. The resulting error is handed to
/// `assert_on_err`, so each call site can layer on its own content assertions
/// (e.g. "must be a transport error, not an auth rejection"); pass a no-op
/// closure when a bare failure is sufficient.
pub fn assert_connect_fails_through_dead_proxy<T, E>(
    attempt: impl FnOnce(u16) -> Result<T, E>,
    assert_on_err: impl FnOnce(&E),
) {
    let dead_port = dead_loopback_port();
    match attempt(dead_port) {
        Ok(_) => panic!("operation through a dead proxy port must fail"),
        Err(err) => assert_on_err(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    // Exercises the fixture's own mechanics (nothing listens on the minted
    // port; `attempt`'s error reaches `assert_on_err`) without needing a real
    // Snowflake account — the live/e2e and hermetic call sites layer
    // account- or mock-GS-specific `attempt`/`assert_on_err` closures on top.
    #[test]
    fn dead_port_rejects_a_direct_connection() {
        let port = dead_loopback_port();
        assert!(
            TcpStream::connect(("127.0.0.1", port)).is_err(),
            "nothing should be listening on a dropped ephemeral port"
        );
    }

    #[test]
    fn assert_connect_fails_through_dead_proxy_passes_err_to_assertion() {
        let mut observed_port = None;
        assert_connect_fails_through_dead_proxy(
            |port| {
                observed_port = Some(port);
                TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())
            },
            |err| assert!(!err.is_empty(), "error message must be non-empty"),
        );
        assert!(
            observed_port.is_some(),
            "attempt must receive the dead port"
        );
    }

    #[test]
    #[should_panic(expected = "operation through a dead proxy port must fail")]
    fn assert_connect_fails_through_dead_proxy_panics_on_unexpected_success() {
        assert_connect_fails_through_dead_proxy(|_port| Ok::<(), String>(()), |_err: &String| {});
    }
}
