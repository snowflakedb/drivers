use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::validator::CrlValidator;
use crate::crl::worker::CrlWorker;
use crate::tls::x509_utils::load_system_root_store;
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
            None => load_system_root_store()
                .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?,
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

        // Build chains, anchor each with rustls, then CRL-check one-by-one
        let inters: Vec<Vec<u8>> = intermediates.iter().map(|c| c.as_ref().to_vec()).collect();

        // All returned chains will be anchored.
        let chains = crate::tls::x509_utils::build_candidate_chains_with_filter(
            end_entity.as_ref(),
            &inters,
            |inters_der: &[rustls::pki_types::CertificateDer<'_>]| verify_path(inters_der).is_ok(),
        );
        if chains.is_empty() {
            return Err(TlsError::General(
                "CRL validation failed: no anchored chains".to_string(),
            ));
        }

        // CRL-check this anchored chain
        let mut saw_chain_revoked = false;
        for chain in chains.iter() {
            let worker = CrlWorker::global();
            let validation = worker.validate(Arc::clone(&self.crl_validator), chain.clone());
            match validation {
                Ok(_) => return Ok(ServerCertVerified::assertion()),
                Err(e) => {
                    if matches!(e, crate::crl::error::CrlError::EndEntityRevoked { .. }) {
                        tracing::error!(target: "sf_core::crl", "CRL validation failed: end-entity certificate revoked");
                        return Err(TlsError::General(
                            "CRL validation failed: end-entity certificate revoked".to_string(),
                        ));
                    }
                    if matches!(e, crate::crl::error::CrlError::ChainRevoked { .. }) {
                        saw_chain_revoked = true;
                    }
                }
            }
        }

        // If we saw a revoked chain, return an error regardless of mode.
        if saw_chain_revoked {
            tracing::error!(target: "sf_core::crl", "CRL validation failed: chain revoked");
            return Err(TlsError::General(
                "CRL validation failed: chain revoked".to_string(),
            ));
        }

        if self.crl_config.check_mode == CertRevocationCheckMode::Advisory {
            tracing::warn!(target: "sf_core::crl", "CRL validation errors but no revoked chain; allowing (advisory)");
            return Ok(ServerCertVerified::assertion());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::cache::CrlCache;
    use crate::tls::revocation::RevocationOutcome;
    use crate::tls::test_helpers::x509 as th;
    use crate::tls::test_helpers::x509::make_root_store;
    use chrono::Utc;

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

        // Seed outcome cache with EE serial using our parser's canonical encoding
        th::clear_all_crl_caches();
        th::seed_revoked(&ee_cert, &inter_cert, 5);

        // Verifier
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store)).unwrap();

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
        th::clear_all_crl_caches();
        th::seed_not_determined(&ee_cert, &inter_cert, 5);
        // Advisory allows
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver =
            CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store.clone())).unwrap();
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
        let ver2 = CrlServerCertVerifier::new_with_root_store(crl_cfg2, Some(root_store)).unwrap();
        let res2 = ver2.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert!(res2.is_err(), "Enabled should fail NotDetermined");
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

        // Seed NotDetermined outcomes for the clean anchored chain (no CRL URLs)
        th::seed_chain_not_determined(&[ee.clone(), inter_b.clone(), inter_a_via_a.clone()], 5);

        // Trust store only contains RootA
        let root_store = make_root_store(&root_a.to_der().unwrap());
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Advisory,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store)).unwrap();

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
        let ver =
            CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store.clone())).unwrap();

        // Seed outcome: Inter revoked under Root
        let cfg = crate::crl::config::CrlConfig {
            enable_memory_caching: true,
            ..Default::default()
        };
        let cache = CrlCache::global(cfg);
        cache.clear_caches_for_tests();
        let future = Utc::now() + chrono::Duration::days(5);
        let inter_serial =
            crate::crl::certificate_parser::get_certificate_serial_number(&inter.to_der().unwrap())
                .unwrap();
        cache.test_put_outcome(
            &inter_serial,
            &root.to_der().unwrap(),
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
        assert!(
            res.is_err(),
            "Advisory should fail when intermediate is revoked"
        );
        // Enabled: fail
        let crl_cfg2 = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            ..Default::default()
        };
        let ver2 = CrlServerCertVerifier::new_with_root_store(crl_cfg2, Some(root_store)).unwrap();
        let res2 = ver2.verify_server_cert(
            &ee_der,
            std::slice::from_ref(&inter_der),
            &server_name,
            &[],
            UnixTime::now(),
        );
        assert!(
            res2.is_err(),
            "Enabled should fail ChainRevoked on intermediate"
        );
    }

    #[test]
    fn cross_signed_chain_anchors_correctly() {
        th::test_setup();
        CrlCache::global(Default::default()).clear_caches_for_tests();
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
        th::seed_not_determined(&ee, &inter_b, 5);
        th::seed_not_determined(&inter_b, &inter_a_via_a, 5);
        th::seed_not_determined(&inter_b, &inter_a_via_b, 5);

        // Trust store only contains RootA
        let root_store = th::make_root_store(&root_a.to_der().unwrap());
        let crl_cfg = crate::crl::config::CrlConfig {
            check_mode: crate::crl::config::CertRevocationCheckMode::Enabled,
            allow_certificates_without_crl_url: true,
            ..Default::default()
        };
        let ver = CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store)).unwrap();

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

        // Seed outcomes: InterB is revoked only under InterA_via_B; clean under InterA_via_A
        let cfg = crate::crl::config::CrlConfig {
            enable_memory_caching: true,
            ..Default::default()
        };
        let cache = CrlCache::global(cfg);
        let future = Utc::now() + chrono::Duration::days(5);
        let inter_b_serial = crate::crl::certificate_parser::get_certificate_serial_number(
            &inter_b.to_der().unwrap(),
        )
        .unwrap();
        cache.test_put_outcome(
            &inter_b_serial,
            &inter_a_via_b.to_der().unwrap(),
            RevocationOutcome::Revoked {
                reason: None,
                revocation_time: None,
            },
            future,
        );
        cache.test_put_outcome(
            &inter_b_serial,
            &inter_a_via_a.to_der().unwrap(),
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
        let ver = CrlServerCertVerifier::new_with_root_store(crl_cfg, Some(root_store)).unwrap();

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
}
