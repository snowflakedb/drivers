//! Background memory timeline monitor using /proc/self/statm (Linux only).
//!
//! Spawns a thread that samples RSS and virtual memory at a configurable
//! interval (default 100 ms). On non-Linux platforms start/stop are no-ops
//! returning an empty timeline.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct MemorySample {
    pub timestamp_ms: u64,
    pub rss_bytes: u64,
    pub vm_bytes: u64,
}

pub struct ResourceMonitor {
    interval: Duration,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Vec<MemorySample>>>,
}

impl ResourceMonitor {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub fn start(&mut self) {
        if !cfg!(target_os = "linux") {
            return;
        }

        self.stop.store(false, Ordering::Relaxed);
        let stop = Arc::clone(&self.stop);
        let interval = self.interval;

        self.thread = Some(std::thread::spawn(move || {
            let mut samples = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                if let Some(s) = take_sample() {
                    samples.push(s);
                }
                std::thread::sleep(interval);
            }
            if let Some(s) = take_sample() {
                samples.push(s);
            }
            samples
        }));
    }

    pub fn stop(&mut self) -> Vec<MemorySample> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread
            .take()
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default()
    }
}

fn take_sample() -> Option<MemorySample> {
    let (rss, vm) = read_statm()?;
    Some(MemorySample {
        timestamp_ms: now_ms(),
        rss_bytes: rss,
        vm_bytes: vm,
    })
}

#[cfg(target_os = "linux")]
fn read_statm() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/self/statm").ok()?;
    let mut parts = content.split_whitespace();
    let vm_pages: u64 = parts.next()?.parse().ok()?;
    let rss_pages: u64 = parts.next()?.parse().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) } as u64;
    Some((rss_pages * page_size, vm_pages * page_size))
}

#[cfg(not(target_os = "linux"))]
fn read_statm() -> Option<(u64, u64)> {
    None
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// CPU time (user + system) for the process in seconds via `getrusage`.
pub fn process_cpu_seconds() -> f64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        timeval_to_secs(&usage.ru_utime) + timeval_to_secs(&usage.ru_stime)
    }
}

/// Process peak RSS in MB. ru_maxrss is KB on Linux, bytes on macOS.
pub fn get_peak_rss_mb() -> f64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        let raw = usage.ru_maxrss as f64;
        if cfg!(target_os = "macos") {
            raw / (1024.0 * 1024.0)
        } else {
            raw / 1024.0
        }
    }
}

fn timeval_to_secs(tv: &libc::timeval) -> f64 {
    tv.tv_sec as f64 + tv.tv_usec as f64 / 1_000_000.0
}
