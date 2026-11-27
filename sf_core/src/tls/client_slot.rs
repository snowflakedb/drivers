use std::future::Future;

use reqwest::Client;
use tokio::sync::Mutex;

use crate::crl::ConnectionCrl;
use crate::tls::client_spec::ClientSpec;
use crate::tls::error::TlsError;

/// The driver's most recently built client and the spec it was built from.
///
/// A lookup with an equal spec, in the process that built the client, returns
/// a clone, so later connections reuse its idle sockets. A different spec or
/// process ID builds a new client and replaces the slot; holders of the
/// previous client keep it. A lookup in a forked child therefore never
/// returns the parent's client. The lock is held across a build, so
/// concurrent lookups with equal specs share one build. A failed build leaves
/// the slot unchanged.
///
/// The client's certificate verifier records `client_crl_stats` into the
/// building connection's buffer, so a lookup returns that buffer with the
/// client. A disabled slot returns the caller's own buffer.
pub(crate) struct ClientSlot {
    enabled: bool,
    cached: Mutex<Option<CachedClient>>,
}

struct CachedClient {
    pid: u32,
    spec: ClientSpec,
    client: Client,
    crl: ConnectionCrl,
}

impl ClientSlot {
    /// A disabled slot builds a new client on every lookup.
    pub(crate) fn new(disabled: bool) -> Self {
        Self {
            enabled: !disabled,
            cached: Mutex::new(None),
        }
    }

    pub(crate) async fn get_or_init<F, Fut>(
        &self,
        spec: ClientSpec,
        connection_crl: &ConnectionCrl,
        build: F,
    ) -> Result<(Client, ConnectionCrl), TlsError>
    where
        F: FnOnce(ClientSpec) -> Fut,
        Fut: Future<Output = Result<Client, TlsError>>,
    {
        if !self.enabled {
            let proxy_mode = spec.proxy.mode();
            tracing::info!(
                proxy_mode,
                cache_enabled = false,
                "building a new HTTP client"
            );
            return Ok((build(spec).await?, connection_crl.clone()));
        }
        self.get_or_init_with(std::process::id(), spec, connection_crl, build)
            .await
    }

    async fn get_or_init_with<F, Fut>(
        &self,
        pid: u32,
        spec: ClientSpec,
        connection_crl: &ConnectionCrl,
        build: F,
    ) -> Result<(Client, ConnectionCrl), TlsError>
    where
        F: FnOnce(ClientSpec) -> Fut,
        Fut: Future<Output = Result<Client, TlsError>>,
    {
        let proxy_mode = spec.proxy.mode();
        let mut cached = self.cached.lock().await;
        if let Some(entry) = cached.as_ref()
            && entry.pid == pid
            && entry.spec == spec
        {
            tracing::info!(proxy_mode, "reusing the cached HTTP client");
            return Ok((entry.client.clone(), entry.crl.clone()));
        }
        tracing::info!(proxy_mode, "building a new HTTP client");
        let client = build(spec.clone()).await?;
        *cached = Some(CachedClient {
            pid,
            spec,
            client: client.clone(),
            crl: connection_crl.clone(),
        });
        Ok((client, connection_crl.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::CrlManager;
    use crate::tls::TlsConfig;
    use crate::tls::client_spec::tests::spec;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_build(
        builds: &AtomicUsize,
    ) -> impl FnOnce(ClientSpec) -> std::future::Ready<Result<Client, TlsError>> + '_ {
        move |_| {
            builds.fetch_add(1, Ordering::Relaxed);
            std::future::ready(Ok(Client::new()))
        }
    }

    fn crl() -> ConnectionCrl {
        CrlManager::new().connection_crl()
    }

    fn default_spec() -> ClientSpec {
        spec(&TlsConfig::default(), None)
    }

    fn other_spec() -> ClientSpec {
        spec(
            &TlsConfig {
                verify_hostname: false,
                ..Default::default()
            },
            None,
        )
    }

    #[tokio::test]
    async fn different_spec_replaces_the_client() {
        let slot = ClientSlot::new(false);
        let builds = AtomicUsize::new(0);
        for spec in [default_spec(), other_spec(), other_spec(), default_spec()] {
            slot.get_or_init_with(1, spec, &crl(), counting_build(&builds))
                .await
                .expect("client");
        }
        assert_eq!(builds.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn different_process_id_builds_its_own_client() {
        let slot = ClientSlot::new(false);
        let builds = AtomicUsize::new(0);
        for _ in 0..2 {
            slot.get_or_init(default_spec(), &crl(), counting_build(&builds))
                .await
                .expect("client");
        }
        let child_pid = std::process::id().wrapping_add(1);
        for _ in 0..2 {
            slot.get_or_init_with(child_pid, default_spec(), &crl(), counting_build(&builds))
                .await
                .expect("client");
        }
        assert_eq!(builds.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn failed_build_leaves_the_slot_unchanged() {
        let slot = ClientSlot::new(false);
        let builds = AtomicUsize::new(0);
        slot.get_or_init_with(1, default_spec(), &crl(), counting_build(&builds))
            .await
            .expect("client");
        let failed = slot
            .get_or_init_with(1, other_spec(), &crl(), |_| {
                std::future::ready(Err(crate::tls::error::InsecureTlsRejectedSnafu.build()))
            })
            .await;
        assert!(failed.is_err());
        slot.get_or_init_with(1, default_spec(), &crl(), counting_build(&builds))
            .await
            .expect("client");
        assert_eq!(builds.load(Ordering::Relaxed), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_first_lookups_share_one_build() {
        let slot = Arc::new(ClientSlot::new(false));
        let builds = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(tokio::sync::Barrier::new(8));
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let slot = Arc::clone(&slot);
                let builds = Arc::clone(&builds);
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    slot.get_or_init_with(1, default_spec(), &crl(), |_| async move {
                        builds.fetch_add(1, Ordering::Relaxed);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        Ok(Client::new())
                    })
                    .await
                })
            })
            .collect();
        for task in tasks {
            task.await.expect("join").expect("client");
        }
        assert_eq!(builds.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn a_reuse_returns_the_building_connections_crl_stats() {
        let slot = ClientSlot::new(false);
        let builder = crl();
        slot.get_or_init_with(1, default_spec(), &builder, |_| {
            std::future::ready(Ok(Client::new()))
        })
        .await
        .expect("client");
        let (_, shared) = slot
            .get_or_init_with(1, default_spec(), &crl(), |_| {
                std::future::ready(Ok(Client::new()))
            })
            .await
            .expect("client");
        builder
            .stats()
            .record("https://snowflake.com/ca.crl", None, None, None, None);
        assert_eq!(shared.pending_crl_stats().len(), 1);
    }
}
