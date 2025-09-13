use x509_parser::prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum X509Error {
    #[error("Failed to parse certificate: {0}")]
    CertParse(String),
    #[error("Failed to parse CRL: {0}")]
    CrlParse(String),
}

pub fn extract_skid(cert_der: &[u8]) -> Result<Option<Vec<u8>>, X509Error> {
    let (_, cert) =
        X509Certificate::from_der(cert_der).map_err(|e| X509Error::CertParse(e.to_string()))?;
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectKeyIdentifier(skid) = ext.parsed_extension() {
            return Ok(Some(skid.0.to_vec()));
        }
    }
    Ok(None)
}

pub fn extract_crl_akid(crl_der: &[u8]) -> Result<Option<Vec<u8>>, X509Error> {
    let (_, crl) = CertificateRevocationList::from_der(crl_der)
        .map_err(|e| X509Error::CrlParse(e.to_string()))?;
    for ext in crl.tbs_cert_list.extensions() {
        if let ParsedExtension::AuthorityKeyIdentifier(akid) = ext.parsed_extension()
            && let Some(key_id) = &akid.key_identifier
        {
            return Ok(Some(key_id.0.to_vec()));
        }
    }
    Ok(None)
}
