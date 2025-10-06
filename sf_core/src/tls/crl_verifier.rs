use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::validator::CrlValidator;
use crate::crl::worker::CrlWorker;
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
        let root_store = Arc::new(root_store);
        let webpki_verifier = WebPkiServerVerifier::builder(root_store.clone()).build()?;
        let crl_validator = Arc::new(CrlValidator::new_with_root_store(
            crl_config.clone(),
            Some(root_store.clone()),
        )?);
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

        let inters: Vec<Vec<u8>> = intermediates.iter().map(|c| c.as_ref().to_vec()).collect();
        let chains = crate::tls::x509_utils::build_candidate_chains(end_entity.as_ref(), &inters);

        let res = CrlWorker::global(Arc::clone(&self.crl_validator)).validate(chains);
        match res {
            Ok(_) => Ok(ServerCertVerified::assertion()),
            Err(e) => match self.crl_config.check_mode {
                CertRevocationCheckMode::Enabled => {
                    tracing::error!(target: "sf_core::crl", error = %e, "CRL validation failed");
                    Err(TlsError::General(format!("CRL validation failed: {e}")))
                }
                CertRevocationCheckMode::Advisory => {
                    tracing::warn!(
                        target: "sf_core::crl",
                        error = %e,
                        "CRL validation failed in advisory mode; allowing connection"
                    );
                    Ok(ServerCertVerified::assertion())
                }
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
