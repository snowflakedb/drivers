use chrono::{DateTime, Utc};
use snafu::{Location, ResultExt, Snafu};
use x509_parser::prelude::*;

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum X509Error {
    #[snafu(display("Failed to parse certificate"))]
    CertParse {
        source: x509_parser::nom::Err<x509_parser::error::X509Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse CRL"))]
    CrlParse {
        source: x509_parser::nom::Err<x509_parser::error::X509Error>,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn extract_skid(cert_der: &[u8]) -> Result<Option<Vec<u8>>, X509Error> {
    let (_, cert) = X509Certificate::from_der(cert_der).context(CertParseSnafu)?;
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectKeyIdentifier(skid) = ext.parsed_extension() {
            return Ok(Some(skid.0.to_vec()));
        }
    }
    Ok(None)
}

pub fn extract_crl_akid(crl_der: &[u8]) -> Result<Option<Vec<u8>>, X509Error> {
    let (_, crl) = CertificateRevocationList::from_der(crl_der).context(CrlParseSnafu)?;
    for ext in crl.tbs_cert_list.extensions() {
        if let ParsedExtension::AuthorityKeyIdentifier(akid) = ext.parsed_extension()
            && let Some(key_id) = &akid.key_identifier
        {
            return Ok(Some(key_id.0.to_vec()));
        }
    }
    Ok(None)
}

pub fn extract_crl_next_update(crl_der: &[u8]) -> Result<Option<DateTime<Utc>>, X509Error> {
    let (_, crl) = CertificateRevocationList::from_der(crl_der).context(CrlParseSnafu)?;
    if let Some(next_update) = crl.tbs_cert_list.next_update {
        if let Some(dt) = crate::crl::certificate_parser::asn1_time_to_datetime(&next_update) {
            return Ok(Some(dt));
        }
        return Ok(None);
    }
    Ok(None)
}

// Best-effort CRL signature verification placeholder
pub fn verify_crl_signature_best_effort(
    _crl_der: &[u8],
    _issuer_der: Option<&[u8]>,
) -> Result<(), X509Error> {
    // TODO: Implement real signature verification; for now assume OK
    Ok(())
}
