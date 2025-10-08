#[cfg(test)]
pub mod x509 {
    #![allow(deprecated)]
    use openssl::asn1::Asn1Time;
    use openssl::hash::MessageDigest;
    use openssl::nid::Nid;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::{X509, X509Extension, X509Name, X509NameBuilder, X509Req, X509ReqBuilder};
    use rustls::RootCertStore;

    // Test-wide setup helpers -------------------------------------------------

    /// Install the rustls CryptoProvider once and clear CRL caches for tests.
    pub fn test_setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
        clear_all_crl_caches();
    }

    pub fn make_name(cn: &str) -> X509Name {
        let mut b = X509NameBuilder::new().unwrap();
        b.append_entry_by_nid(Nid::COMMONNAME, cn).unwrap();
        b.build()
    }

    // Note: CRL builders in openssl crate are limited; tests should prefer fixture-based CRLs

    pub fn make_root_store(root_der: &[u8]) -> rustls::RootCertStore {
        use rustls::pki_types::CertificateDer;
        let mut store = rustls::RootCertStore::empty();
        let certs = vec![CertificateDer::from(root_der.to_vec())];
        let (_added, _ignored) = store.add_parsable_certificates(certs);
        store
    }

    /// Build a RootCertStore from multiple X509 certificates.
    pub fn make_root_store_from(certs: &[X509]) -> RootCertStore {
        use rustls::pki_types::CertificateDer;
        let mut store = RootCertStore::empty();
        let ders: Vec<CertificateDer<'static>> = certs
            .iter()
            .map(|c| CertificateDer::from(c.to_der().unwrap()))
            .collect();
        let (_added, _ignored) = store.add_parsable_certificates(ders);
        store
    }

    /// Add multiple X509 roots to an existing RootCertStore.
    pub fn add_roots(store: &mut RootCertStore, certs: &[X509]) {
        use rustls::pki_types::CertificateDer;
        let ders: Vec<CertificateDer<'static>> = certs
            .iter()
            .map(|c| CertificateDer::from(c.to_der().unwrap()))
            .collect();
        let _ = store.add_parsable_certificates(ders);
    }
    pub fn gen_key() -> PKey<openssl::pkey::Private> {
        let rsa = Rsa::generate(2048).unwrap();
        PKey::from_rsa(rsa).unwrap()
    }

    pub fn gen_req(subject_cn: &str, key: &PKey<openssl::pkey::Private>) -> X509Req {
        let mut rb = X509ReqBuilder::new().unwrap();
        rb.set_subject_name(&make_name(subject_cn)).unwrap();
        rb.set_pubkey(key).unwrap();
        rb.sign(key, MessageDigest::sha256()).unwrap();
        rb.build()
    }

    pub fn sign_cert(
        req: &X509Req,
        issuer_name: &openssl::x509::X509NameRef,
        issuer_key: &PKey<openssl::pkey::Private>,
        is_ca: bool,
    ) -> X509 {
        let mut builder = X509::builder().unwrap();
        builder.set_version(2).unwrap();
        builder.set_subject_name(req.subject_name()).unwrap();
        builder.set_issuer_name(issuer_name).unwrap();
        builder
            .set_pubkey(req.public_key().as_ref().unwrap())
            .unwrap();
        let nb = Asn1Time::days_from_now(0).unwrap();
        let na = Asn1Time::days_from_now(if is_ca { 365 } else { 30 }).unwrap();
        builder.set_not_before(&nb).unwrap();
        builder.set_not_after(&na).unwrap();
        let bc_val = if is_ca { "CA:TRUE" } else { "CA:FALSE" };
        let bc = X509Extension::new_nid(
            None,
            Some(&builder.x509v3_context(None, None)),
            Nid::BASIC_CONSTRAINTS,
            bc_val,
        )
        .unwrap();
        builder.append_extension(bc).unwrap();
        if !is_ca {
            let san = X509Extension::new_nid(
                None,
                Some(&builder.x509v3_context(None, None)),
                Nid::SUBJECT_ALT_NAME,
                "DNS:test.example.com",
            )
            .unwrap();
            builder.append_extension(san).unwrap();
        }
        builder.sign(issuer_key, MessageDigest::sha256()).unwrap();
        builder.build()
    }

    // DER and serial helpers --------------------------------------------------

    /// Convert a slice of X509 to rustls CertificateDer for verifier inputs.
    pub fn to_cert_der_vec(certs: &[X509]) -> Vec<rustls::pki_types::CertificateDer<'static>> {
        certs
            .iter()
            .map(|c| rustls::pki_types::CertificateDer::from(c.to_der().unwrap()))
            .collect()
    }

    /// Extract canonical serial number bytes of a certificate for CRL cache keys.
    pub fn serial_of(cert: &X509) -> Vec<u8> {
        crate::crl::certificate_parser::get_certificate_serial_number(&cert.to_der().unwrap())
            .unwrap()
    }

    // CRL outcome and cache helpers ------------------------------------------

    /// Clear all in-memory CRL caches used in tests.
    pub fn clear_all_crl_caches() {
        let cache = crate::crl::cache::CrlCache::global(Default::default());
        cache.clear_caches_for_tests();
    }

    /// Seed a Revoked outcome for (subject, issuer) with a short TTL.
    pub fn seed_revoked(subject: &X509, issuer: &X509, ttl_days: i64) {
        use crate::tls::revocation::RevocationOutcome;
        let cache = crate::crl::cache::CrlCache::global(crate::crl::config::CrlConfig {
            enable_memory_caching: true,
            ..Default::default()
        });
        let until = chrono::Utc::now() + chrono::Duration::days(ttl_days);
        let serial = serial_of(subject);
        cache.test_put_outcome(
            &serial,
            &issuer.to_der().unwrap(),
            RevocationOutcome::Revoked {
                reason: None,
                revocation_time: None,
            },
            until,
        );
    }

    /// Seed a NotDetermined outcome for (subject, issuer) with a short TTL.
    pub fn seed_not_determined(subject: &X509, issuer: &X509, ttl_days: i64) {
        use crate::tls::revocation::RevocationOutcome;
        let cache = crate::crl::cache::CrlCache::global(crate::crl::config::CrlConfig {
            enable_memory_caching: true,
            ..Default::default()
        });
        let until = chrono::Utc::now() + chrono::Duration::days(ttl_days);
        let serial = serial_of(subject);
        cache.test_put_outcome(
            &serial,
            &issuer.to_der().unwrap(),
            RevocationOutcome::NotDetermined,
            until,
        );
    }

    /// Seed NotDetermined for each adjacent pair in a chain ordered as [EE, Inter1, ..., Top].
    pub fn seed_chain_not_determined(chain: &[X509], ttl_days: i64) {
        for win in chain.windows(2) {
            let subj = &win[0];
            let iss = &win[1];
            seed_not_determined(subj, iss, ttl_days);
        }
    }
}
