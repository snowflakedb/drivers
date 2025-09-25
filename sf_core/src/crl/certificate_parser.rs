use crate::crl::error::{CrlError, CrlParsingSnafu};
use snafu::Location;
use snafu::ResultExt;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::*;

pub fn extract_crl_distribution_points(cert_der: &[u8]) -> Result<Vec<String>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;

    let mut crl_urls = Vec::new();
    for ext in cert.extensions() {
        if ext.oid == x509_parser::oid_registry::OID_X509_EXT_CRL_DISTRIBUTION_POINTS
            && let ParsedExtension::CRLDistributionPoints(crl_dist_points) = &ext.parsed_extension()
        {
            for dist_point in &crl_dist_points.points {
                if let Some(x509_parser::extensions::DistributionPointName::FullName(
                    general_names,
                )) = &dist_point.distribution_point
                {
                    for general_name in general_names {
                        if let GeneralName::URI(uri) = general_name {
                            let url = uri.to_string();
                            if url.starts_with("http://") || url.starts_with("https://") {
                                crl_urls.push(url);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(crl_urls)
}

pub fn is_short_lived_certificate(cert_der: &[u8]) -> Result<bool, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;

    let validity = &cert.validity;
    let not_before_dt =
        asn1_time_to_datetime(&validity.not_before).ok_or_else(|| CrlError::CrlParsing {
            source: x509_parser::nom::Err::Failure(x509_parser::error::X509Error::InvalidDate),
            location: Location::new(file!(), line!(), 0),
        })?;
    let not_after_dt =
        asn1_time_to_datetime(&validity.not_after).ok_or_else(|| CrlError::CrlParsing {
            source: x509_parser::nom::Err::Failure(x509_parser::error::X509Error::InvalidDate),
            location: Location::new(file!(), line!(), 0),
        })?;

    let validity_period_seconds = (not_after_dt - not_before_dt).num_seconds();
    let seven_days_seconds = 7 * 24 * 60 * 60;
    Ok(validity_period_seconds <= seven_days_seconds)
}

pub fn get_certificate_serial_number(cert_der: &[u8]) -> Result<Vec<u8>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    Ok(cert.serial.to_bytes_be())
}

pub fn check_certificate_in_crl(cert_serial: &[u8], crl_der: &[u8]) -> Result<bool, CrlError> {
    use x509_parser::revocation_list::CertificateRevocationList;
    let (_, crl) = CertificateRevocationList::from_der(crl_der).context(CrlParsingSnafu)?;

    let now = chrono::Utc::now();
    if let Some(next_update) = crl.tbs_cert_list.next_update {
        let next_update_time =
            asn1_time_to_datetime(&next_update).ok_or_else(|| CrlError::CrlExpired {
                location: Location::new(file!(), line!(), 0),
            })?;
        if now > next_update_time {
            return Err(CrlError::CrlExpired {
                location: Location::new(file!(), line!(), 0),
            });
        }
    }
    for revoked_cert in crl.iter_revoked_certificates() {
        if revoked_cert.raw_serial() == cert_serial {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn asn1_time_to_datetime(
    asn1_time: &x509_parser::time::ASN1Time,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let dt = asn1_time.to_datetime();
    let ts = dt.unix_timestamp();
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
}
