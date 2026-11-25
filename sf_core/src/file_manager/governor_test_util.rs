use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::Notify;

use crate::governor::{Governor, TransferOp};

pub(super) async fn notified_soon(notify: &Notify) {
    tokio::time::timeout(Duration::from_secs(2), notify.notified())
        .await
        .expect("timed out waiting for notify");
}

pub(super) async fn wait_until(mut cond: impl FnMut() -> bool) {
    let start = tokio::time::Instant::now();
    while !cond() {
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "timed out waiting for governor condition"
        );
        tokio::task::yield_now().await;
    }
}

pub(super) async fn start_stalled_206_range_server() -> (String, Arc<Notify>, Arc<Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled 206 server");
    let address = listener.local_addr().expect("stalled 206 address");
    let first_headers = Arc::new(Notify::new());
    let release_first_body = Arc::new(Notify::new());
    tokio::spawn({
        let first_headers = first_headers.clone();
        let release_first_body = release_first_body.clone();
        async move {
            let (mut first, _) = listener.accept().await.expect("accept range GET A");
            let mut request = vec![0; 4096];
            let _ = first.read(&mut request).await.expect("read range GET A");
            first
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/4\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("write range GET A headers");
            first_headers.notify_one();
            release_first_body.notified().await;
            first
                .write_all(b"abcd")
                .await
                .expect("write range GET A body");
            drop(first);

            let (mut second, _) = listener.accept().await.expect("accept range GET B");
            let _ = second.read(&mut request).await.expect("read range GET B");
            second
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/4\r\nConnection: close\r\n\r\nabcd",
                )
                .await
                .expect("respond range GET B");
        }
    });
    (
        format!("http://{address}"),
        first_headers,
        release_first_body,
    )
}

pub(super) async fn wait_for_arrivals(arrivals: &AtomicUsize, want: usize) -> usize {
    let deadline = Duration::from_secs(10);
    tokio::time::timeout(deadline, async {
        loop {
            let seen = arrivals.load(Ordering::SeqCst);
            if seen >= want {
                return seen;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "expected at least {want} concurrent transfers within {deadline:?}, saw {}",
            arrivals.load(Ordering::SeqCst)
        )
    })
}

pub(super) async fn wait_for_arrival_peak(
    arrivals: &AtomicUsize,
    governor: &Governor,
    want: usize,
) -> (usize, usize) {
    wait_for_arrival_peak_with_commands(arrivals, governor, want, &[])
        .await
        .0
}

pub(super) async fn wait_for_arrival_peak_with_commands(
    arrivals: &AtomicUsize,
    governor: &Governor,
    want: usize,
    commands: &[TransferOp],
) -> ((usize, usize), Vec<usize>) {
    let deadline = Duration::from_secs(10);
    let mut peak_arrivals = 0usize;
    let mut peak_in_flight = 0usize;
    let mut peak_command: Vec<usize> = vec![0; commands.len()];
    tokio::time::timeout(deadline, async {
        loop {
            let seen = arrivals.load(Ordering::SeqCst);
            peak_arrivals = peak_arrivals.max(seen);
            peak_in_flight = peak_in_flight.max(governor.in_flight());
            for (idx, op) in commands.iter().enumerate() {
                peak_command[idx] = peak_command[idx].max(op.command_in_flight());
            }
            if seen >= want {
                tokio::time::sleep(Duration::from_millis(50)).await;
                peak_arrivals = peak_arrivals.max(arrivals.load(Ordering::SeqCst));
                peak_in_flight = peak_in_flight.max(governor.in_flight());
                for (idx, op) in commands.iter().enumerate() {
                    peak_command[idx] = peak_command[idx].max(op.command_in_flight());
                }
                return ((peak_arrivals, peak_in_flight), peak_command);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "expected at least {want} concurrent transfers within {deadline:?}, saw {} \
             arrivals and {} in_flight",
            arrivals.load(Ordering::SeqCst),
            governor.in_flight()
        )
    })
}
