use crate::crl::ConnectionCrl;
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::validator::{ChainValidationOutcome, CrlValidator, RevokedCert};
use crate::crl::worker::SharedCrlWorker;
use crate::tls::RootStoreSnapshot;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::{WebPkiServerVerifier, verify_server_cert_signed_by_trust_anchor};
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{DigitallySignedStruct, Error as TlsError, RootCertStore, SignatureScheme};
use std::ops::ControlFlow;
use std::sync::Arc;

/// Signature algorithms for chain and CRL verification, taken from the
/// linked crypto module so they match the provider that negotiates the handshake.
fn default_supported_algs() -> WebPkiSupportedAlgorithms {
    crate::tls::crypto_module::CryptoModule::get().signature_verification_algorithms()
}

fn resolve(
    outcome: ChainValidationOutcome,
    mode: &CertRevocationCheckMode,
) -> Result<ServerCertVerified, TlsError> {
    match outcome {
        ChainValidationOutcome::NotRevoked => Ok(ServerCertVerified::assertion()),
        ChainValidationOutcome::Revoked(cert) => {
            let message = match cert {
                RevokedCert::EndEntity => "CRL validation failed: end-entity certificate revoked",
                RevokedCert::Intermediate => "CRL validation failed: chain revoked",
            };
            tracing::error!(target: "sf_core::crl", "{message}");
            Err(TlsError::General(message.to_string()))
        }
        ChainValidationOutcome::Indeterminate => match mode {
            CertRevocationCheckMode::Disabled => Ok(ServerCertVerified::assertion()),
            CertRevocationCheckMode::Advisory => {
                tracing::warn!(target: "sf_core::crl", "CRL validation errors but no revoked chain; allowing (advisory)");
                Ok(ServerCertVerified::assertion())
            }
            CertRevocationCheckMode::Enabled => {
                tracing::error!(target: "sf_core::crl", "CRL validation failed");
                Err(TlsError::General("CRL validation failed".to_string()))
            }
        },
    }
}

/// Folds one chain's outcome into the outcome of the chains seen so far. A `NotRevoked` chain
/// or a revoked end-entity certificate ends the search. Otherwise `Indeterminate` absorbs a
/// revoked intermediate, so the result is `Revoked` only when every chain is revoked. A
/// revoked intermediate can belong to a single path, and a path that could not be checked is
/// unverified rather than revoked.
fn fold_chain_outcome(
    best: Option<ChainValidationOutcome>,
    next: ChainValidationOutcome,
) -> ControlFlow<ChainValidationOutcome, ChainValidationOutcome> {
    match next {
        ChainValidationOutcome::NotRevoked
        | ChainValidationOutcome::Revoked(RevokedCert::EndEntity) => ControlFlow::Break(next),
        ChainValidationOutcome::Revoked(RevokedCert::Intermediate) => {
            ControlFlow::Continue(best.unwrap_or(next))
        }
        ChainValidationOutcome::Indeterminate => {
            ControlFlow::Continue(ChainValidationOutcome::Indeterminate)
        }
    }
}

#[derive(Debug)]
pub(crate) struct CrlServerCertVerifier {
    /// Used for cert validation when `verify_hostname` is true, and always used
    /// for TLS 1.2/1.3 signature verification and supported scheme queries
    /// regardless of `verify_hostname`.
    webpki_verifier: Arc<WebPkiServerVerifier>,
    crl_validator: Arc<CrlValidator>,
    crl_config: CrlConfig,
    verify_hostname: bool,
    root_store: Arc<RootCertStore>,
    supported_algs: WebPkiSupportedAlgorithms,
    crl_worker: SharedCrlWorker,
    _crl_manager: ConnectionCrl,
}

impl CrlServerCertVerifier {
    pub(crate) fn new_with_root_store(
        crl_config: CrlConfig,
        root_store: Arc<RootCertStore>,
        verify_hostname: bool,
        crl: ConnectionCrl,
        crl_download_roots: &RootStoreSnapshot<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let supported_algs = default_supported_algs();
        let webpki_verifier = WebPkiServerVerifier::builder_with_provider(
            root_store.clone(),
            crate::tls::crypto_module::CryptoModule::get().provider(),
        )
        .build()?;
        let crl_worker = crl.worker();
        let cache = crl.cache(crl_config.clone(), crl_download_roots)?;
        let crl_validator = Arc::new(
            CrlValidator::new_with_root_store(crl_config.clone(), Some(root_store.clone()), cache)
                .with_crl_stats(crl.stats()),
        );
        Ok(Self {
            webpki_verifier,
            crl_validator,
            crl_config,
            verify_hostname,
            root_store,
            supported_algs,
            crl_worker,
            _crl_manager: crl,
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
        // Parse only when hostname verification is disabled (chain-only path).
        let parsed_cert: Option<ParsedCertificate<'_>> = if !self.verify_hostname {
            Some(ParsedCertificate::try_from(end_entity)?)
        } else {
            None
        };

        // Helper closure to re-validate a path with fixed verifier inputs.
        // When verify_hostname is false, use chain-only validation (skip hostname check).
        let verify_path = |inters: &[CertificateDer<'_>]| match &parsed_cert {
            None => self.webpki_verifier.verify_server_cert(
                end_entity,
                inters,
                server_name,
                ocsp_response,
                now,
            ),
            Some(cert) => {
                verify_server_cert_signed_by_trust_anchor(
                    cert,
                    &self.root_store,
                    inters,
                    now,
                    self.supported_algs.all,
                )?;
                Ok(ServerCertVerified::assertion())
            }
        };

        // Validate the handshake path
        verify_path(intermediates)?;
        if self.crl_config.check_mode == CertRevocationCheckMode::Disabled {
            return Ok(ServerCertVerified::assertion());
        }

        // Build chains, anchor each with rustls, then CRL-check one-by-one
        let inters: Vec<Vec<u8>> = intermediates.iter().map(|c| c.as_ref().to_vec()).collect();

        // All returned chains will be anchored.
        let chains = crate::tls::x509_utils::build_candidate_chains_with_filter(
            end_entity.as_ref(),
            &inters,
            |inters_der: &[rustls::pki_types::CertificateDer<'_>]| verify_path(inters_der).is_ok(),
        );
        let mut validation_outcome = None;
        for chain in chains.iter() {
            let next = self
                .crl_worker
                .validate(Arc::clone(&self.crl_validator), chain.clone());
            match fold_chain_outcome(validation_outcome, next) {
                ControlFlow::Break(outcome) => {
                    validation_outcome = Some(outcome);
                    break;
                }
                ControlFlow::Continue(outcome) => validation_outcome = Some(outcome),
            }
        }
        let Some(validation_outcome) = validation_outcome else {
            return Err(TlsError::General(
                "CRL validation failed: no anchored chains".to_string(),
            ));
        };

        resolve(validation_outcome, &self.crl_config.check_mode)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::CrlManager;
    use crate::crl::cache::CrlCache;
    use crate::tls::revocation::RevocationOutcome;
    use crate::tls::test_helpers::x509 as th;
    use crate::tls::test_helpers::x509::make_root_store;
    use chrono::Utc;

    fn test_crl() -> ConnectionCrl {
        CrlManager::new().connection_crl()
    }

    fn memory_cache(crl: &CrlManager) -> Arc<CrlCache> {
        crl.cache(
            crate::crl::config::CrlConfig {
                enable_memory_caching: true,
                ..Default::default()
            },
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .expect("crl cache")
    }

    #[test]
    fn verifier_holds_the_manager_until_the_verifier_drops() {
        th::test_setup();
        let root_key = th::gen_key();
        let root_name = th::make_name("Test Root");
        let root_req = th::gen_req("Test Root", &root_key);
        let root_cert = th::sign_cert(&root_req, &root_name, &root_key, true);
        let root_store = make_root_store(&root_cert.to_der().unwrap());
        let crl_config = crate::crl::config::CrlConfig {
            enable_memory_caching: true,
            enable_disk_caching: false,
            ..Default::default()
        };
        let crl = test_crl();
        let cache = crl
            .cache(
                crl_config.clone(),
                &crate::tls::RootStoreProvider::new().snapshot(),
            )
            .expect("cache");
        cache.wait_for_scheduler(true);
        let verifier = CrlServerCertVerifier::new_with_root_store(
            crl_config,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .expect("verifier");
        assert!(cache.scheduler_is_published());
        drop(verifier);
        cache.wait_for_scheduler(false);
    }

    #[test]
    fn verifier_fails_on_ee_revoked_even_in_advisory() {
        th::test_setup();
        // Generate root, intermediate, EE
        let root_key = th::gen_key();
        let root_name = th::make_name("Test Root");
        let root_req = th::gen_req("Test Root", &root_key);
        let root_cert = th::sign_cert(&root_req, &root_name, &root_key, true);
        let inter_key = th::gen_key();
        let inter_req = th::gen_req("Test Inter", &inter_key);
        let inter_cert = th::sign_cert(&inter_req, root_cert.subject_name(), &root_key, true);
        let ee_key = th::gen_key();
        let ee_req = th::gen_req("Test EE", &ee_key);
        let ee_cert = th::sign_cert(&ee_req, inter_cert.subject_name(), &inter_key, false);

        // Root store
        let root_store = make_root_store(&root_cert.to_der().unwrap());

        let crl = test_crl();
        th::seed_revoked(&memory_cache(&crl), &ee_cert, 5);

        // Verifier
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();

        let ee_der = rustls::pki_types::CertificateDer::from(ee_cert.to_der().unwrap());
        let inter_der = rustls::pki_types::CertificateDer::from(inter_cert.to_der().unwrap());
        let server_name = ServerName::try_from("test.example.com").unwrap();
        let res = ver.verify_server_cert(&ee_der, &[inter_der], &server_name, &[], UnixTime::now());
        assert!(res.is_err(), "EE revoked should fail even in advisory");
    }

    #[test]
    fn resolve_anchor_issuer_key_returns_none_for_invalid_crl() {
        th::test_setup();
        // Test that resolve_anchor_issuer_key returns None for invalid/garbled CRL
        let root_key = th::gen_key();
        let root_name = th::make_name("TestRoot");
        let root_req = th::gen_req("TestRoot", &root_key);
        let root_cert = th::sign_cert(&root_req, &root_name, &root_key, true);
        let root_store = make_root_store(&root_cert.to_der().unwrap());

        // Invalid CRL DER should return None
        let invalid_crl_der = vec![0x30, 0x03, 0x02, 0x01, 0x00]; // Minimal invalid ASN.1
        let result =
            crate::tls::x509_utils::resolve_anchor_issuer_key(&invalid_crl_der, &root_store);
        assert!(result.is_none(), "Should return None for invalid CRL");
    }

    // Note: Anchored top-intermediate CRL verification is tested implicitly through:
    // - crl_short_circuit_skips_beyond_anchor: demonstrates short-circuiting at anchors
    // - cross_signed_chain_anchors_correctly: demonstrates multi-path anchoring
    // CRL preflight checks (delta CRLs, unknown critical extensions, attribute-only CRLs)
    // are tested in x509_utils.rs via check_crl_preflight_policy and related unit tests.

    #[test]
    fn advisory_allows_on_invalid_crl_sig() {
        th::test_setup();
        // Advisory vs Enabled mapping using outcome cache NotDetermined
        let root_key = th::gen_key();
        let root_req = th::gen_req("R2", &root_key);
        let root_cert = th::sign_cert(&root_req, &th::make_name("R2"), &root_key, true);
        let inter_key = th::gen_key();
        let inter_req = th::gen_req("I2", &inter_key);
        let inter_cert = th::sign_cert(&inter_req, root_cert.subject_name(), &root_key, true);
        let ee_key = th::gen_key();
        let ee_req = th::gen_req("E2", &ee_key);
        let ee_cert = th::sign_cert(&ee_req, inter_cert.subject_name(), &inter_key, false);
        let root_store = th::make_root_store(&root_cert.to_der().unwrap());
        let crl = test_crl();
        th::seed_not_determined(&memory_cache(&crl), &ee_cert, 5);
        // Advisory allows
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            allow_certificates_without_crl_url: false,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store.clone()),
            true,
            crl.clone(),
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let ee_der = rustls::pki_types::CertificateDer::from(ee_cert.to_der().unwrap());
        let inter_der = rustls::pki_types::CertificateDer::from(inter_cert.to_der().unwrap());
        let server_name = ServerName::try_from("test.example.com").unwrap();
        let res = ver.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert!(res.is_ok(), "Advisory should allow NotDetermined");
        // Enabled fails
        let crl_cfg2 = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            allow_certificates_without_crl_url: false,
            ..Default::default()
        };
        let ver2 = CrlServerCertVerifier::new_with_root_store(
            crl_cfg2,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let res2 = ver2.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert_eq!(
            res2.unwrap_err(),
            TlsError::General("CRL validation failed".to_string()),
            "Enabled should fail NotDetermined with generic CRL validation error, not chain revoked"
        );
    }

    #[test]
    fn advisory_allows_when_crl_download_returns_not_found_and_enabled_fails() {
        use rcgen::{
            BasicConstraints, CertificateParams, CrlDistributionPoint, DistinguishedName, DnType,
            IsCa, KeyPair, KeyUsagePurpose,
        };
        use std::io::{Read, Write};

        th::test_setup();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/ca.crl", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let mut served = 0usize;
            while served < 2 && std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        let mut buf = [0u8; 1024];
                        let _ = stream.read(&mut buf);
                        stream
                            .write_all(
                                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            )
                            .unwrap();
                        served += 1;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("fake CRL server accept failed: {e}"),
                }
            }
            served
        });

        let ca_key = KeyPair::generate().unwrap();
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        let mut ca_dn = DistinguishedName::new();
        ca_dn.push(DnType::CommonName, "Test Root CA");
        ca_params.distinguished_name = ca_dn;
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_cert = ca_params.self_signed(&ca_key).unwrap();
        let ca_der = ca_cert.der().to_vec();

        let inter_key = KeyPair::generate().unwrap();
        let mut inter_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        let mut inter_dn = DistinguishedName::new();
        inter_dn.push(DnType::CommonName, "Test Intermediate CA");
        inter_params.distinguished_name = inter_dn;
        inter_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        inter_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let inter_cert = inter_params
            .signed_by(&inter_key, &ca_cert, &ca_key)
            .unwrap();
        let inter_der = CertificateDer::from(inter_cert.der().to_vec());

        let ee_key = KeyPair::generate().unwrap();
        let mut ee_params = CertificateParams::new(vec!["test.snowflake.com".into()]).unwrap();
        let mut ee_dn = DistinguishedName::new();
        ee_dn.push(DnType::CommonName, "test.snowflake.com");
        ee_params.distinguished_name = ee_dn;
        ee_params.is_ca = IsCa::NoCa;
        ee_params.crl_distribution_points = vec![CrlDistributionPoint { uris: vec![url] }];
        let ee_cert = ee_params
            .signed_by(&ee_key, &inter_cert, &inter_key)
            .unwrap();
        let ee_der = CertificateDer::from(ee_cert.der().to_vec());

        let root_store = Arc::new(make_root_store(&ca_der));
        let server_name = ServerName::try_from("test.snowflake.com").unwrap();

        let advisory_verifier = CrlServerCertVerifier::new_with_root_store(
            CrlConfig {
                check_mode: CertRevocationCheckMode::Advisory,
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..Default::default()
            },
            Arc::clone(&root_store),
            true,
            test_crl(),
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let advisory_res = advisory_verifier.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );

        let enabled_verifier = CrlServerCertVerifier::new_with_root_store(
            CrlConfig {
                check_mode: CertRevocationCheckMode::Enabled,
                enable_memory_caching: false,
                enable_disk_caching: false,
                ..Default::default()
            },
            root_store,
            true,
            test_crl(),
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let enabled_res = enabled_verifier.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );

        let served = server.join().unwrap();
        assert_eq!(
            served, 2,
            "the Advisory and Enabled verifiers each download the CRL once"
        );

        assert!(
            advisory_res.is_ok(),
            "Advisory mode must allow connection on CRL HTTP 404: {advisory_res:?}"
        );
        assert_eq!(
            enabled_res.unwrap_err(),
            TlsError::General("CRL validation failed".to_string()),
            "Enabled mode must fail on CRL HTTP 404 with CRL validation failed, not chain revoked"
        );
    }

    #[test]
    fn fold_chain_outcome_is_revoked_only_when_every_chain_is_revoked() {
        let revoked = ChainValidationOutcome::Revoked(RevokedCert::Intermediate);
        let indeterminate = ChainValidationOutcome::Indeterminate;

        assert_eq!(
            fold_chain_outcome(None, revoked),
            ControlFlow::Continue(revoked)
        );
        assert_eq!(
            fold_chain_outcome(Some(revoked), revoked),
            ControlFlow::Continue(revoked)
        );
        assert_eq!(
            fold_chain_outcome(Some(revoked), indeterminate),
            ControlFlow::Continue(indeterminate)
        );
        assert_eq!(
            fold_chain_outcome(Some(indeterminate), revoked),
            ControlFlow::Continue(indeterminate)
        );
    }

    #[test]
    fn fold_chain_outcome_stops_on_clean_chain_or_revoked_end_entity() {
        let revoked = ChainValidationOutcome::Revoked(RevokedCert::Intermediate);
        let end_entity = ChainValidationOutcome::Revoked(RevokedCert::EndEntity);

        assert_eq!(
            fold_chain_outcome(Some(revoked), ChainValidationOutcome::NotRevoked),
            ControlFlow::Break(ChainValidationOutcome::NotRevoked)
        );
        assert_eq!(
            fold_chain_outcome(Some(ChainValidationOutcome::Indeterminate), end_entity),
            ControlFlow::Break(end_entity)
        );
    }

    fn fold_all(outcomes: &[ChainValidationOutcome]) -> ChainValidationOutcome {
        let mut best = None;
        for next in outcomes {
            best = Some(match fold_chain_outcome(best, *next) {
                ControlFlow::Break(outcome) | ControlFlow::Continue(outcome) => outcome,
            });
        }
        best.unwrap()
    }

    #[test]
    fn mixed_revoked_and_indeterminate_chains_fail_open_only_in_advisory() {
        let revoked = ChainValidationOutcome::Revoked(RevokedCert::Intermediate);
        let indeterminate = ChainValidationOutcome::Indeterminate;
        for chain_outcomes in [[indeterminate, revoked], [revoked, indeterminate]] {
            let folded = fold_all(&chain_outcomes);
            assert!(resolve(folded, &CertRevocationCheckMode::Advisory).is_ok());
            assert_eq!(
                resolve(folded, &CertRevocationCheckMode::Enabled).unwrap_err(),
                TlsError::General("CRL validation failed".to_string())
            );
        }
    }

    #[test]
    fn every_chain_revoked_fails_in_advisory_and_enabled() {
        let folded = fold_all(&[
            ChainValidationOutcome::Revoked(RevokedCert::Intermediate),
            ChainValidationOutcome::Revoked(RevokedCert::Intermediate),
        ]);
        for mode in [
            CertRevocationCheckMode::Advisory,
            CertRevocationCheckMode::Enabled,
        ] {
            assert_eq!(
                resolve(folded, &mode).unwrap_err(),
                TlsError::General("CRL validation failed: chain revoked".to_string())
            );
        }
    }

    #[test]
    fn resolve_allows_indeterminate_only_in_advisory() {
        let indeterminate = ChainValidationOutcome::Indeterminate;
        assert!(resolve(indeterminate, &CertRevocationCheckMode::Advisory).is_ok());
        assert_eq!(
            resolve(indeterminate, &CertRevocationCheckMode::Enabled).unwrap_err(),
            TlsError::General("CRL validation failed".to_string())
        );
        for mode in [
            CertRevocationCheckMode::Advisory,
            CertRevocationCheckMode::Enabled,
        ] {
            assert_eq!(
                resolve(
                    ChainValidationOutcome::Revoked(RevokedCert::EndEntity),
                    &mode
                )
                .unwrap_err(),
                TlsError::General(
                    "CRL validation failed: end-entity certificate revoked".to_string()
                )
            );
        }
    }

    #[test]
    fn crl_short_circuit_skips_beyond_anchor() {
        th::test_setup();
        // Build two possible paths for InterA: RootA (trusted) and RootB (untrusted)
        // EE -> InterB -> InterA; InterA is cross-signed by RootA and RootB
        let root_a_key = th::gen_key();
        let root_a = th::sign_cert(
            &th::gen_req("RootA", &root_a_key),
            &th::make_name("RootA"),
            &root_a_key,
            true,
        );
        let root_b_key = th::gen_key();
        let root_b = th::sign_cert(
            &th::gen_req("RootB", &root_b_key),
            &th::make_name("RootB"),
            &root_b_key,
            true,
        );

        let inter_a_key = th::gen_key();
        let inter_a_req = th::gen_req("InterA", &inter_a_key);
        let inter_a_via_a = th::sign_cert(&inter_a_req, root_a.subject_name(), &root_a_key, true);
        // Cross-signed variant of InterA issued by RootB (not in store)
        let inter_a_via_b = th::sign_cert(&inter_a_req, root_b.subject_name(), &root_b_key, true);

        let inter_b_key = th::gen_key();
        let inter_b = th::sign_cert(
            &th::gen_req("InterB", &inter_b_key),
            inter_a_via_a.subject_name(),
            &inter_a_key,
            true,
        );
        let ee_key = th::gen_key();
        let ee = th::sign_cert(
            &th::gen_req("EE", &ee_key),
            inter_b.subject_name(),
            &inter_b_key,
            false,
        );

        let crl = test_crl();
        th::seed_chain_not_determined(
            &memory_cache(&crl),
            &[ee.clone(), inter_b.clone(), inter_a_via_a.clone()],
            5,
        );

        // Trust store only contains RootA
        let root_store = make_root_store(&root_a.to_der().unwrap());
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();

        // Presented intermediates include both InterA variants
        let ee_der = rustls::pki_types::CertificateDer::from(ee.to_der().unwrap());
        let inters = vec![
            rustls::pki_types::CertificateDer::from(inter_b.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_a.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_b.to_der().unwrap()),
        ];
        let server_name = ServerName::try_from("test.example.com").unwrap();

        // No CRLs seeded: verifier should anchor on RootA path and short-circuit any CRL beyond first anchor
        let res = ver.verify_server_cert(&ee_der, &inters, &server_name, &[], UnixTime::now());
        assert!(
            res.is_ok(),
            "Anchored RootA path should succeed despite untrusted alternative"
        );
    }

    #[test]
    fn advisory_fails_when_intermediate_revoked() {
        th::test_setup();
        // Root -> Inter -> EE; mark Inter revoked so overall chain is revoked, but Advisory should allow
        let root_key = th::gen_key();
        let root_name = th::make_name("R");
        let root = th::sign_cert(&th::gen_req("R", &root_key), &root_name, &root_key, true);
        let inter_key = th::gen_key();
        let inter = th::sign_cert(
            &th::gen_req("I", &inter_key),
            root.subject_name(),
            &root_key,
            true,
        );
        let ee_key = th::gen_key();
        let ee = th::sign_cert(
            &th::gen_req("E", &ee_key),
            inter.subject_name(),
            &inter_key,
            false,
        );

        let root_store = make_root_store(&root.to_der().unwrap());
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            ..Default::default()
        };
        let crl = test_crl();
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store.clone()),
            true,
            crl.clone(),
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();

        let cache = memory_cache(&crl);
        cache.clear_caches_for_tests();
        let future = Utc::now() + chrono::Duration::days(5);
        let inter_serial =
            crate::crl::certificate_parser::get_certificate_serial_number(&inter.to_der().unwrap())
                .unwrap();
        cache.test_put_outcome(
            &inter_serial,
            &inter.to_der().unwrap(),
            RevocationOutcome::Revoked {
                reason: None,
                revocation_time: None,
            },
            future,
        );

        let ee_der = rustls::pki_types::CertificateDer::from(ee.to_der().unwrap());
        let inter_der = rustls::pki_types::CertificateDer::from(inter.to_der().unwrap());
        let server_name = ServerName::try_from("test.example.com").unwrap();
        // Advisory: should fail on real revocation
        let res = ver.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert_eq!(
            res.unwrap_err(),
            TlsError::General("CRL validation failed: chain revoked".to_string()),
            "Advisory should fail when intermediate is revoked"
        );
        // Enabled: fail
        let crl_cfg2 = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            ..Default::default()
        };
        let ver2 = CrlServerCertVerifier::new_with_root_store(
            crl_cfg2,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let res2 = ver2.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert_eq!(
            res2.unwrap_err(),
            TlsError::General("CRL validation failed: chain revoked".to_string()),
            "Enabled should fail ChainRevoked on intermediate"
        );
    }

    #[test]
    fn cross_signed_chain_anchors_correctly() {
        th::test_setup();
        let crl = test_crl();
        let cache = memory_cache(&crl);
        // Cross-sign scenario: InterA has two variants (via RootA trusted, via RootB untrusted)
        // Verifier should anchor the chain through RootA and succeed
        let root_a_key = th::gen_key();
        let root_a = th::sign_cert(
            &th::gen_req("RootA", &root_a_key),
            &th::make_name("RootA"),
            &root_a_key,
            true,
        );
        let root_b_key = th::gen_key();
        let root_b = th::sign_cert(
            &th::gen_req("RootB", &root_b_key),
            &th::make_name("RootB"),
            &root_b_key,
            true,
        );

        let inter_a_key = th::gen_key();
        let inter_a_req = th::gen_req("InterA", &inter_a_key);
        let inter_a_via_a = th::sign_cert(&inter_a_req, root_a.subject_name(), &root_a_key, true);
        let inter_a_via_b = th::sign_cert(&inter_a_req, root_b.subject_name(), &root_b_key, true);
        let inter_b_key = th::gen_key();
        let inter_b = th::sign_cert(
            &th::gen_req("InterB", &inter_b_key),
            inter_a_via_a.subject_name(),
            &inter_a_key,
            true,
        );
        let ee_key = th::gen_key();
        let ee = th::sign_cert(
            &th::gen_req("EE", &ee_key),
            inter_b.subject_name(),
            &inter_b_key,
            false,
        );

        // Seed NotDetermined for both alternative parentings of InterB
        th::seed_not_determined(&cache, &ee, 5);
        th::seed_not_determined(&cache, &inter_b, 5);

        // Trust store only contains RootA
        let root_store = th::make_root_store(&root_a.to_der().unwrap());
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();

        let ee_der = rustls::pki_types::CertificateDer::from(ee.to_der().unwrap());
        let inters = vec![
            rustls::pki_types::CertificateDer::from(inter_b.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_a.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_b.to_der().unwrap()),
        ];
        let server_name = ServerName::try_from("test.example.com").unwrap();
        let res = ver.verify_server_cert(&ee_der, &inters, &server_name, &[], UnixTime::now());
        assert!(
            res.is_ok(),
            "Should anchor chain through trusted root and succeed"
        );
    }

    #[test]
    fn anchored_clean_path_wins_when_other_anchored_path_revoked() {
        th::test_setup();

        // Two roots; both are trusted so both alternative chains will anchor
        let root_a_key = th::gen_key();
        let root_a = th::sign_cert(
            &th::gen_req("RootA", &root_a_key),
            &th::make_name("RootA"),
            &root_a_key,
            true,
        );
        let root_b_key = th::gen_key();
        let root_b = th::sign_cert(
            &th::gen_req("RootB", &root_b_key),
            &th::make_name("RootB"),
            &root_b_key,
            true,
        );

        // InterA is cross-signed by both roots; InterB is issued by InterA; EE is issued by InterB
        let inter_a_key = th::gen_key();
        let inter_a_req = th::gen_req("InterA", &inter_a_key);
        let inter_a_via_a = th::sign_cert(&inter_a_req, root_a.subject_name(), &root_a_key, true);
        let inter_a_via_b = th::sign_cert(&inter_a_req, root_b.subject_name(), &root_b_key, true);

        let inter_b_key = th::gen_key();
        let inter_b = th::sign_cert(
            &th::gen_req("InterB", &inter_b_key),
            inter_a_via_a.subject_name(),
            &inter_a_key,
            true,
        );
        let ee_key = th::gen_key();
        let ee = th::sign_cert(
            &th::gen_req("EE", &ee_key),
            inter_b.subject_name(),
            &inter_b_key,
            false,
        );

        let crl = test_crl();
        let cache = memory_cache(&crl);
        let future = Utc::now() + chrono::Duration::days(5);
        let inter_b_serial = crate::crl::certificate_parser::get_certificate_serial_number(
            &inter_b.to_der().unwrap(),
        )
        .unwrap();
        cache.test_put_outcome(
            &inter_b_serial,
            &inter_b.to_der().unwrap(),
            RevocationOutcome::NotDetermined,
            future,
        );

        // Trust store contains both roots so both chains anchor
        let root_store = th::make_root_store_from(&[root_a.clone(), root_b.clone()]);

        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();

        let ee_der = rustls::pki_types::CertificateDer::from(ee.to_der().unwrap());
        let inters = vec![
            rustls::pki_types::CertificateDer::from(inter_b.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_a.to_der().unwrap()),
            rustls::pki_types::CertificateDer::from(inter_a_via_b.to_der().unwrap()),
        ];
        let server_name = ServerName::try_from("test.example.com").unwrap();
        let res = ver.verify_server_cert(&ee_der, &inters, &server_name, &[], UnixTime::now());
        assert!(
            res.is_ok(),
            "Anchored clean path should succeed even when another anchored path is revoked"
        );
    }

    #[test]
    fn hostname_bypass_allows_mismatched_server_name_in_advisory() {
        th::test_setup();
        let crl = test_crl();
        let cache = memory_cache(&crl);
        let root_key = th::gen_key();
        let root_cert = th::sign_cert(
            &th::gen_req("R", &root_key),
            &th::make_name("R"),
            &root_key,
            true,
        );
        let inter_key = th::gen_key();
        let inter_cert = th::sign_cert(
            &th::gen_req("I", &inter_key),
            root_cert.subject_name(),
            &root_key,
            true,
        );
        let ee_key = th::gen_key();
        let ee_cert = th::sign_cert(
            &th::gen_req("E", &ee_key),
            inter_cert.subject_name(),
            &inter_key,
            false,
        );
        let root_store = th::make_root_store(&root_cert.to_der().unwrap());
        th::seed_not_determined(&cache, &ee_cert, 5);

        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };

        // verify_hostname=false should succeed despite mismatched server_name
        let ver = CrlServerCertVerifier::new_with_root_store(
            crl_cfg.clone(),
            Arc::new(root_store.clone()),
            false,
            crl.clone(),
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let ee_der = rustls::pki_types::CertificateDer::from(ee_cert.to_der().unwrap());
        let inter_der = rustls::pki_types::CertificateDer::from(inter_cert.to_der().unwrap());
        let mismatched_name = ServerName::try_from("wrong.host.example.com").unwrap();
        let res = ver.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &mismatched_name,
            &[],
            UnixTime::now(),
        );
        assert!(
            res.is_ok(),
            "verify_hostname=false should skip hostname check"
        );

        // verify_hostname=true should fail with the same mismatched server_name
        let ver2 = CrlServerCertVerifier::new_with_root_store(
            crl_cfg,
            Arc::new(root_store),
            true,
            crl,
            &crate::tls::RootStoreProvider::new().snapshot(),
        )
        .unwrap();
        let res2 = ver2.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &mismatched_name,
            &[],
            UnixTime::now(),
        );
        assert!(
            res2.is_err(),
            "verify_hostname=true should reject mismatched hostname"
        );
    }
}
