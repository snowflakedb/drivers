use crate::crl::error::CrlError;
use crate::crl::validator::CrlValidator;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

pub struct CrlWorkerRequest {
    pub chain: Vec<Vec<u8>>,
    pub reply: mpsc::Sender<Result<(), CrlError>>,
}

pub struct CrlWorker {
    tx: Sender<CrlWorkerRequest>,
}

static GLOBAL_WORKER: OnceCell<CrlWorker> = OnceCell::new();

impl CrlWorker {
    pub fn global(validator: Arc<CrlValidator>) -> &'static CrlWorker {
        GLOBAL_WORKER.get_or_init(|| {
            let (tx, rx): (Sender<CrlWorkerRequest>, Receiver<CrlWorkerRequest>) = mpsc::channel();

            std::thread::Builder::new()
                .name("crl-worker".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create CRL worker runtime");

                    rt.block_on(async move {
                        while let Ok(req) = rx.recv() {
                            let res = match validator.validate_certificate_chain(&req.chain).await {
                                Ok(true) => Ok(()),
                                Ok(false) => Err(CrlError::AllChainsRevoked {
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
        })
    }

    pub fn validate(&self, chain: Vec<Vec<u8>>) -> Result<(), CrlError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let msg = CrlWorkerRequest {
            chain,
            reply: reply_tx,
        };
        self.tx.send(msg).expect("CRL worker channel closed");
        reply_rx.recv().expect("CRL worker reply channel closed")
    }
}
