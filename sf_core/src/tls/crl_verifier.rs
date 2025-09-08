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
    pub fn new(crl_config: CrlConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_root_store(crl_config, None)
    }

    pub fn new_with_root_store(
        crl_config: CrlConfig,
        custom_root_store: Option<rustls::RootCertStore>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create root store - use custom if provided, otherwise use webpki defaults
        let root_store = match custom_root_store {
            Some(custom_store) => {
                tracing::info!("Using custom root certificate store for CRL validation");
                custom_store
            }
            None => {
                tracing::debug!("Using default webpki root certificate store for CRL validation");
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
        tracing::debug!("TLS certificate verification with CRL validation");
        tracing::debug!("Server name: {:?}", server_name);
        tracing::debug!(
            "Certificate chain length: {} (1 end entity + {} intermediates)",
            1 + intermediates.len(),
            intermediates.len()
        );

        // First, perform standard certificate validation
        tracing::debug!("Performing standard certificate validation");
        self.webpki_verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;

        tracing::debug!("Standard certificate validation passed");

        // If CRL validation is disabled, we're done
        if self.crl_config.check_mode == CertRevocationCheckMode::Disabled {
            tracing::debug!("CRL validation disabled, skipping revocation check");
            return Ok(ServerCertVerified::assertion());
        }

        // Perform CRL validation
        tracing::debug!("Performing CRL revocation validation");

        // Convert certificate chain to the format expected by CRL validator
        let mut cert_chain = Vec::with_capacity(1 + intermediates.len());
        cert_chain.push(end_entity.as_ref().to_vec());
        for intermediate in intermediates {
            cert_chain.push(intermediate.as_ref().to_vec());
        }

        // Create certificate chains (we have one chain)
        let cert_chains = vec![cert_chain];

        // Dispatch CRL validation to the global worker and block for the result
        let worker = CrlWorker::global(self.crl_validator.clone());
        let crl_result = worker.validate(cert_chains);

        match crl_result {
            Ok(()) => {
                tracing::debug!("CRL validation passed - certificates are not revoked");
                Ok(ServerCertVerified::assertion())
            }
            Err(e) => match self.crl_config.check_mode {
                CertRevocationCheckMode::Enabled => {
                    tracing::error!("CRL validation failed in ENABLED mode: {}", e);
                    Err(TlsError::General(format!(
                        "Certificate revocation check failed: {}",
                        e
                    )))
                }
                CertRevocationCheckMode::Advisory => {
                    tracing::warn!(
                        "CRL validation failed in ADVISORY mode, allowing connection: {}",
                        e
                    );
                    Ok(ServerCertVerified::assertion())
                }
                CertRevocationCheckMode::Disabled => unreachable!(),
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
