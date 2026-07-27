use crate::crl::error::CrlError;
use crate::crl::validator::CrlValidator;
use std::fmt;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::mpsc::{self, Receiver, Sender};

pub struct CrlWorkerRequest {
    pub chain: Vec<Vec<u8>>,
    pub validator: Arc<CrlValidator>,
    pub reply: mpsc::Sender<Result<(), CrlError>>,
}

pub struct CrlWorker {
    tx: Sender<CrlWorkerRequest>,
}

/// Shareable lazy CRL worker handle. The background thread starts on first use
/// inside a CRL-enabled TLS path, not when the handle is created.
pub type SharedCrlWorker = Arc<LazyLock<CrlWorker>>;

impl fmt::Debug for CrlWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CrlWorker")
    }
}

impl CrlWorker {
    /// Spawns the background `"crl-worker"` thread. Invoked only via [`LazyLock`]
    /// inside [`SharedCrlWorker`].
    fn spawn() -> Self {
        let (tx, rx): (Sender<CrlWorkerRequest>, Receiver<CrlWorkerRequest>) = mpsc::channel();

        let dispatch = tracing::dispatcher::get_default(|d| d.clone());
        std::thread::Builder::new()
            .name("crl-worker".into())
            .spawn(move || {
                let _log_guard = tracing::dispatcher::set_default(&dispatch);
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create CRL worker runtime");

                rt.block_on(async move {
                    while let Ok(req) = rx.recv() {
                        let res = match req.validator.validate_certificate_chain(&req.chain).await {
                            Ok(true) => Ok(()),
                            Ok(false) => Err(CrlError::ChainRevoked {
                                location: snafu::Location::new(file!(), line!(), 0),
                            }),
                            Err(e) => Err(e),
                        };
                        let _ = req.reply.send(res);
                    }
                });
            })
            .expect("Failed to spawn CRL worker thread");

        CrlWorker { tx }
    }

    /// Returns a lazy [`SharedCrlWorker`] for standalone TLS clients (dev binaries,
    /// integration tests). Production wrappers should use
    /// [`DatabaseDriverV1::crl_worker`](crate::apis::database_driver_v1::DatabaseDriverV1::crl_worker).
    pub fn shared_lazy() -> SharedCrlWorker {
        Arc::new(LazyLock::new(Self::spawn))
    }

    /// Same as [`shared_lazy`](Self::shared_lazy); used when constructing
    /// [`DatabaseDriverV1`](crate::apis::database_driver_v1::DatabaseDriverV1).
    pub(crate) fn new_lazy() -> SharedCrlWorker {
        Self::shared_lazy()
    }

    pub fn validate(
        &self,
        validator: Arc<CrlValidator>,
        chain: Vec<Vec<u8>>,
    ) -> Result<(), CrlError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let msg = CrlWorkerRequest {
            chain,
            validator,
            reply: reply_tx,
        };
        self.tx.send(msg).expect("CRL worker channel closed");
        reply_rx.recv().expect("CRL worker reply channel closed")
    }
}
