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

    /// Install the rustls CryptoProvider once.
    pub fn test_setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
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

    /// Seed a Revoked outcome for `subject`, keyed by its issuer DN, with a short TTL.
    pub fn seed_revoked(cache: &crate::crl::cache::CrlCache, subject: &X509, ttl_days: i64) {
        use crate::tls::revocation::RevocationOutcome;
        let until = chrono::Utc::now() + chrono::Duration::days(ttl_days);
        let serial = serial_of(subject);
        cache.test_put_outcome(
            &serial,
            &subject.to_der().unwrap(),
            RevocationOutcome::Revoked {
                reason: None,
                revocation_time: None,
            },
            until,
        );
    }

    /// Seed a NotDetermined outcome for `subject`, keyed by its issuer DN, with a short TTL.
    pub fn seed_not_determined(cache: &crate::crl::cache::CrlCache, subject: &X509, ttl_days: i64) {
        use crate::tls::revocation::RevocationOutcome;
        let until = chrono::Utc::now() + chrono::Duration::days(ttl_days);
        let serial = serial_of(subject);
        cache.test_put_outcome(
            &serial,
            &subject.to_der().unwrap(),
            RevocationOutcome::NotDetermined,
            until,
        );
    }

    /// Seed NotDetermined for each adjacent pair in a chain ordered as [EE, Inter1, ..., Top].
    pub fn seed_chain_not_determined(
        cache: &crate::crl::cache::CrlCache,
        chain: &[X509],
        ttl_days: i64,
    ) {
        for subject in chain.iter().take(chain.len().saturating_sub(1)) {
            seed_not_determined(cache, subject, ttl_days);
        }
    }
}

#[cfg(test)]
pub mod tls_server {
    use rcgen::{
        BasicConstraints, CertificateParams, CertificateRevocationListParams, CrlDistributionPoint,
        DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose, RevokedCertParams, SerialNumber,
    };
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::io::Write;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::NamedTempFile;
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const REVOKED_SERIAL: u64 = 0xDEAD_BEEF;

    pub struct TlsTestServer {
        port: u16,
        handshakes: Arc<AtomicUsize>,
    }

    impl TlsTestServer {
        pub async fn spawn(
            chain: Vec<CertificateDer<'static>>,
            key: PrivateKeyDer<'static>,
        ) -> Self {
            super::x509::test_setup();
            let server_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(chain, key)
                .unwrap();
            let acceptor = TlsAcceptor::from(Arc::new(server_config));
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let handshakes = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&handshakes);
            tokio::spawn(async move {
                while let Ok((tcp, _)) = listener.accept().await {
                    counter.fetch_add(1, Ordering::SeqCst);
                    let _ = acceptor.accept(tcp).await;
                }
            });
            Self { port, handshakes }
        }

        pub async fn spawn_self_signed() -> Self {
            let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
            Self::spawn(
                vec![CertificateDer::from(cert.cert)],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der())),
            )
            .await
        }

        pub fn url(&self, path: &str) -> String {
            format!("https://localhost:{}{path}", self.port)
        }

        pub fn handshakes(&self) -> usize {
            self.handshakes.load(Ordering::SeqCst)
        }
    }

    /// Root CA -> intermediate CA -> `localhost` leaf, where the leaf's serial
    /// is listed in the intermediate's CRL served from `crl_server`.
    pub struct RevokedLeafFixture {
        pub crl_server: MockServer,
        pub server: TlsTestServer,
        root_pem: NamedTempFile,
    }

    impl RevokedLeafFixture {
        pub async fn start() -> Self {
            let ca_key = KeyPair::generate().unwrap();
            let ca_cert = ca_params("Test Root CA").self_signed(&ca_key).unwrap();

            let inter_key = KeyPair::generate().unwrap();
            let inter_cert = ca_params("Test Intermediate CA")
                .signed_by(&inter_key, &ca_cert, &ca_key)
                .unwrap();

            let now = time::OffsetDateTime::now_utc();
            let crl_der = CertificateRevocationListParams {
                this_update: now,
                next_update: now + time::Duration::days(7),
                crl_number: SerialNumber::from(1u64),
                issuing_distribution_point: None,
                revoked_certs: vec![RevokedCertParams {
                    serial_number: SerialNumber::from(REVOKED_SERIAL),
                    revocation_time: now,
                    reason_code: None,
                    invalidity_date: None,
                }],
                key_identifier_method: rcgen::KeyIdMethod::Sha256,
            }
            .signed_by(&inter_cert, &inter_key)
            .unwrap()
            .der()
            .to_vec();

            let crl_server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "application/pkix-crl")
                        .set_body_bytes(crl_der),
                )
                .mount(&crl_server)
                .await;

            let leaf_key = KeyPair::generate().unwrap();
            let mut leaf_params = CertificateParams::new(vec!["localhost".into()]).unwrap();
            leaf_params.distinguished_name = common_name("localhost");
            leaf_params.is_ca = IsCa::NoCa;
            leaf_params.serial_number = Some(SerialNumber::from(REVOKED_SERIAL));
            leaf_params.crl_distribution_points = vec![CrlDistributionPoint {
                uris: vec![format!("{}/test-ca.crl", crl_server.uri())],
            }];
            let leaf_cert = leaf_params
                .signed_by(&leaf_key, &inter_cert, &inter_key)
                .unwrap();

            let server = TlsTestServer::spawn(
                vec![
                    CertificateDer::from(leaf_cert),
                    CertificateDer::from(inter_cert),
                ],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
            )
            .await;

            let mut root_pem = NamedTempFile::new().unwrap();
            root_pem.write_all(ca_cert.pem().as_bytes()).unwrap();
            root_pem.flush().unwrap();

            Self {
                crl_server,
                server,
                root_pem,
            }
        }

        pub fn root_pem_path(&self) -> &Path {
            self.root_pem.path()
        }

        pub async fn crl_downloads(&self) -> usize {
            self.crl_server.received_requests().await.unwrap().len()
        }
    }

    fn common_name(cn: &str) -> DistinguishedName {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, cn);
        dn
    }

    fn ca_params(cn: &str) -> CertificateParams {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.distinguished_name = common_name(cn);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params
    }
}
