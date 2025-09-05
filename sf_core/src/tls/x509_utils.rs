// Small helpers to centralize dual x509 crate usage
use crate::crl::error::CrlError;
use x509_cert::crl::CertificateList as RcCertificateList;
use x509_cert::der::{Decode, Encode};

/// Return canonical DER of the CRL's TBS (to-be-signed) part
pub fn tbs_crl_der(crl_der: &[u8]) -> Result<Vec<u8>, CrlError> {
    let crl = RcCertificateList::from_der(crl_der).map_err(|_| CrlError::InvalidCrlSignature {
        location: snafu::Location::new(file!(), line!(), 0),
    })?;
    crl.tbs_cert_list
        .to_der()
        .map_err(|_| CrlError::InvalidCrlSignature {
            location: snafu::Location::new(file!(), line!(), 0),
        })
}

/// Extract thisUpdate and nextUpdate from a CRL, converted to chrono
pub fn crl_times(
    crl_der: &[u8],
) -> Result<
    (
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
    CrlError,
> {
    use x509_parser::prelude::FromDer;
    let (_, crl) = x509_parser::revocation_list::CertificateRevocationList::from_der(crl_der)
        .map_err(|e| CrlError::CrlParsing {
            source: e.into(),
            location: snafu::Location::new(file!(), line!(), 0),
        })?;
    let this_dt =
        crate::crl::certificate_parser::asn1_time_to_datetime(&crl.tbs_cert_list.this_update)
            .ok_or_else(|| CrlError::CrlParsing {
                source: x509_parser::error::X509Error::InvalidDate,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
    let next_dt_opt = match crl.tbs_cert_list.next_update {
        Some(ref n) => Some(
            crate::crl::certificate_parser::asn1_time_to_datetime(n).ok_or_else(|| {
                CrlError::CrlParsing {
                    source: x509_parser::error::X509Error::InvalidDate,
                    location: snafu::Location::new(file!(), line!(), 0),
                }
            })?,
        ),
        None => None,
    };
    Ok((this_dt, next_dt_opt))
}
