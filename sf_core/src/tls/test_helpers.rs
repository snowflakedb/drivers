#[cfg(test)]
pub mod x509 {
    #![allow(deprecated)]
    use openssl::asn1::Asn1Time;
    use openssl::hash::MessageDigest;
    use openssl::nid::Nid;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::{X509, X509Extension, X509Name, X509NameBuilder, X509Req, X509ReqBuilder};

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
}
