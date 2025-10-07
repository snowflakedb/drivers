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
        // Helper closure to re-validate a path with fixed verifier inputs
        let verify_path = |inters: &[rustls::pki_types::CertificateDer<'_>]| {
            self.webpki_verifier.verify_server_cert(
                end_entity,
                inters,
                server_name,
                ocsp_response,
                now,
            )
        };

        // Validate the handshake path
        verify_path(intermediates)?;
        if self.crl_config.check_mode == CertRevocationCheckMode::Disabled {
            return Ok(ServerCertVerified::assertion());
        }

        // Build chains, anchor each with webpki, then CRL-check one-by-one
        let worker = CrlWorker::global(Arc::clone(&self.crl_validator));
        let mut last_err: Option<crate::crl::error::CrlError> = None;
        let inters: Vec<Vec<u8>> = intermediates.iter().map(|c| c.as_ref().to_vec()).collect();

        // Chains vector will always contain at least one chain, and all chains will have at least one element.
        let chains = crate::tls::x509_utils::build_candidate_chains(end_entity.as_ref(), &inters);
        for chain in chains.iter() {
            // Re-validate this exact path anchors with webpki/rustls
            let inters_der = to_cert_der(&chain[1..]);
            if verify_path(&inters_der).is_err() {
                continue;
            }

            // CRL-check this anchored chain
            match worker.validate(chain.clone()) {
                Ok(_) => return Ok(ServerCertVerified::assertion()),
                Err(e) => {
                    if matches!(e, crate::crl::error::CrlError::EndEntityRevoked { .. }) {
                        tracing::error!(target: "sf_core::crl", "CRL validation failed: end-entity certificate revoked");
                        return Err(TlsError::General(
                            "CRL validation failed: end-entity certificate revoked".to_string(),
                        ));
                    }
                    last_err = Some(e)
                }
            }
        }

        // No anchored-and-unrevoked chain found
        if self.crl_config.check_mode == CertRevocationCheckMode::Advisory {
            if let Some(e) = last_err {
                tracing::warn!(
                    target: "sf_core::crl",
                    error = %e,
                    "CRL validation failed in advisory mode; allowing connection"
                );
            }
            return Ok(ServerCertVerified::assertion());
        }
        if let Some(e) = last_err {
            tracing::error!(target: "sf_core::crl", error = %e, "CRL validation failed");
            return Err(TlsError::General(format!("CRL validation failed: {e}")));
        }
        tracing::error!(target: "sf_core::crl", "CRL validation failed");
        Err(TlsError::General("CRL validation failed".to_string()))
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

// Convert a slice of DER byte vectors into rustls CertificateDer wrappers (zero-copy)
fn to_cert_der<'a>(ders: &'a [Vec<u8>]) -> Vec<rustls::pki_types::CertificateDer<'a>> {
    ders.iter()
        .map(|v| rustls::pki_types::CertificateDer::from(v.as_slice()))
        .collect()
}
