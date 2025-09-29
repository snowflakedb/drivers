// Real TLS integration with CRL validation using rustls
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::validator::CrlValidator;
use crate::crl::worker::CrlWorker;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};
use std::sync::Arc;

/// Custom certificate verifier that integrates CRL validation with TLS handshake
#[derive(Debug)]
pub struct CrlServerCertVerifier {
    /// Standard webpki verifier for basic certificate validation
    webpki_verifier: Arc<WebPkiServerVerifier>,
    /// CRL validator for revocation checking
    crl_validator: Arc<CrlValidator>,
    /// CRL configuration
    crl_config: CrlConfig,
}

impl CrlServerCertVerifier {
    pub fn new_with_root_store(
        crl_config: CrlConfig,
        custom_root_store: Option<rustls::RootCertStore>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create root store - use custom if provided, otherwise use webpki defaults
        let root_store = match custom_root_store {
            Some(custom_store) => custom_store,
            None => {
                let mut root_store = rustls::RootCertStore::empty();
                root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                root_store
            }
        };

        let webpki_verifier = WebPkiServerVerifier::builder(Arc::new(root_store)).build()?;

        // Create CRL validator
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
        // Always perform baseline verification first
        self.webpki_verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;

        // Respect mode; Disabled => no CRL check
        if self.crl_config.check_mode == CertRevocationCheckMode::Disabled {
            return Ok(ServerCertVerified::assertion());
        }

        // Build a single chain for the CRL validator
        let mut chain = Vec::with_capacity(1 + intermediates.len());
        chain.push(end_entity.as_ref().to_vec());
        for i in intermediates {
            chain.push(i.as_ref().to_vec());
        }
        let chains = vec![chain];
        let worker = CrlWorker::global(self.crl_validator.clone());
        let res = worker.validate(chains);

        match res {
            Ok(()) => Ok(ServerCertVerified::assertion()),
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
        // Delegate to webpki verifier
        self.webpki_verifier
            .verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        // Delegate to webpki verifier
        self.webpki_verifier
            .verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Delegate to webpki verifier
        self.webpki_verifier.supported_verify_schemes()
    }
}
