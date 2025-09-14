use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::validator::CrlValidator;
use lazy_static::lazy_static;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};
use std::sync::Arc;

#[derive(Debug)]
pub struct CrlServerCertVerifier {
    webpki_verifier: Arc<WebPkiServerVerifier>,
    crl_validator: Arc<CrlValidator>,
    crl_config: CrlConfig,
}

impl CrlServerCertVerifier {
    pub fn new_with_root_store(
        crl_config: CrlConfig,
        custom_root_store: Option<rustls::RootCertStore>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let root_store = match custom_root_store {
            Some(store) => store,
            None => {
                let mut s = rustls::RootCertStore::empty();
                s.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                s
            }
        };
        let webpki_verifier = WebPkiServerVerifier::builder(Arc::new(root_store)).build()?;
        let crl_validator = Arc::new(CrlValidator::new(crl_config.clone())?);
        Ok(Self {
            webpki_verifier,
            crl_validator,
            crl_config,
        })
    }
}

impl ServerCertVerifier for CrlServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        self.webpki_verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;
        if self.crl_config.check_mode == CertRevocationCheckMode::Disabled {
            return Ok(ServerCertVerified::assertion());
        }

        let mut chain = Vec::with_capacity(1 + intermediates.len());
        chain.push(end_entity.as_ref().to_vec());
        for i in intermediates {
            chain.push(i.as_ref().to_vec());
        }
        let chains = vec![chain];
        // Use shared Tokio runtime to avoid per-handshake runtimes
        let res = shared_runtime().block_on(async {
            self.crl_validator
                .validate_certificate_chains(&chains)
                .await
        });
        match res {
            Ok(_) => Ok(ServerCertVerified::assertion()),
            Err(_) => match self.crl_config.check_mode {
                CertRevocationCheckMode::Enabled => {
                    Err(TlsError::General("CRL validation failed".to_string()))
                }
                CertRevocationCheckMode::Advisory => Ok(ServerCertVerified::assertion()),
                CertRevocationCheckMode::Disabled => Ok(ServerCertVerified::assertion()),
            },
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        self.webpki_verifier
            .verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        self.webpki_verifier
            .verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.webpki_verifier.supported_verify_schemes()
    }
}

fn shared_runtime() -> &'static tokio::runtime::Runtime {
    lazy_static! {
        static ref SHARED_RUNTIME: tokio::runtime::Runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                // Fall back to a minimal runtime to avoid panic in TLS handshake path
                tracing::error!(target: "sf_core::tls", "Failed to create shared CRL runtime: {e}");
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
            }
        };
    }
    &SHARED_RUNTIME
}
