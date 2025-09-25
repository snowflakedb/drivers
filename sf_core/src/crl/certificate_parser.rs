use crate::crl::error::{CrlError, CrlParsingSnafu};
use snafu::Location;
use snafu::ResultExt;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::*;

/// Extract CRL distribution points from a DER-encoded certificate
pub fn extract_crl_distribution_points(cert_der: &[u8]) -> Result<Vec<String>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;

    let mut crl_urls = Vec::new();
    for ext in cert.extensions() {
        if ext.oid == x509_parser::oid_registry::OID_X509_EXT_CRL_DISTRIBUTION_POINTS {
            match &ext.parsed_extension() {
                ParsedExtension::CRLDistributionPoints(crl_dist_points) => {
                    for dist_point in &crl_dist_points.points {
                        if let Some(dist_point_name) = &dist_point.distribution_point {
                            match dist_point_name {
                                x509_parser::extensions::DistributionPointName::FullName(
                                    general_names,
                                ) => {
                                    for general_name in general_names {
                                        if let GeneralName::URI(uri) = general_name {
                                            let url = uri.to_string();
                                            // Per RFC 5280 and CA/B BRs, CRL DP URIs are HTTP endpoints (http/https).
                                            if url.starts_with("http://")
                                                || url.starts_with("https://")
                                            {
                                                crl_urls.push(url);
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // Handle other distribution point name types if needed
                                    tracing::debug!("Unsupported distribution point name type");
                                }
                            }
                        }
                    }
                }
                _ => {
                    tracing::debug!("CRL Distribution Points extension present but not parsed");
                }
            }
        }
    }

    // Return empty list if none found; policy is handled by validator
    if crl_urls.is_empty() {
        tracing::debug!("No CRL distribution points found in certificate");
        return Ok(Vec::new());
    }

    let count = crl_urls.len();
    tracing::debug!("Found {count} CRL distribution points: {crl_urls:?}");
    Ok(crl_urls)
}

/// Check if a certificate is short-lived (validity period <= 7 days)
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
    let seven_days_seconds = 7 * 24 * 60 * 60; // 7 days in seconds

    let is_short_lived = validity_period_seconds <= seven_days_seconds;

    if is_short_lived {
        let days = validity_period_seconds / (24 * 60 * 60);
        tracing::info!("Certificate is short-lived ({days} days), skipping CRL check");
    }

    Ok(is_short_lived)
}

/// Get certificate serial number as bytes for CRL comparison
pub fn get_certificate_serial_number(cert_der: &[u8]) -> Result<Vec<u8>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    Ok(cert.serial.to_bytes_be())
}

/// Parse and validate a CRL, checking if a certificate serial number is revoked
pub fn check_certificate_in_crl(cert_serial: &[u8], crl_der: &[u8]) -> Result<bool, CrlError> {
    use x509_parser::revocation_list::CertificateRevocationList;
    let (_, crl) = CertificateRevocationList::from_der(crl_der).context(CrlParsingSnafu)?;

    // Check if the CRL has expired
    let now = chrono::Utc::now();
    if let Some(next_update) = crl.tbs_cert_list.next_update {
        let next_update_time =
            asn1_time_to_datetime(&next_update).ok_or_else(|| CrlError::CrlExpired {
                location: Location::new(file!(), line!(), 0),
            })?;

        if now > next_update_time {
            tracing::warn!("CRL has expired (next update was {next_update_time})");
            return Err(CrlError::CrlExpired {
                location: Location::new(file!(), line!(), 0),
            });
        }
    }

    // Check if certificate is in the revoked list
    for revoked_cert in crl.iter_revoked_certificates() {
        if revoked_cert.raw_serial() == cert_serial {
            let serial_hex = hex::encode(cert_serial);
            tracing::warn!("Certificate with serial {serial_hex} found in CRL revocation list");
            return Ok(true); // Certificate is revoked
        }
    }

    let serial_hex = hex::encode(cert_serial);
    tracing::debug!("Certificate with serial {serial_hex} not found in CRL revocation list");
    Ok(false) // Certificate is not revoked
}

/// Convert ASN.1 time to chrono DateTime
pub(crate) fn asn1_time_to_datetime(
    asn1_time: &x509_parser::time::ASN1Time,
) -> Option<chrono::DateTime<chrono::Utc>> {
    // x509-parser exposes to_datetime() returning time::OffsetDateTime in recent versions
    let dt = asn1_time.to_datetime(); // time::OffsetDateTime
    let ts = dt.unix_timestamp();
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_crl_distribution_points_empty_cert() {
        // Test with invalid certificate data
        let invalid_cert = vec![0x00, 0x01, 0x02];
        let result = extract_crl_distribution_points(&invalid_cert);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_short_lived_certificate_invalid() {
        let invalid_cert = vec![0x00, 0x01, 0x02];
        let result = is_short_lived_certificate(&invalid_cert);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_certificate_serial_number_invalid() {
        let invalid_cert = vec![0x00, 0x01, 0x02];
        let result = get_certificate_serial_number(&invalid_cert);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_certificate_in_crl_invalid() {
        let cert_serial = vec![0x01, 0x02, 0x03];
        let invalid_crl = vec![0x00, 0x01, 0x02];
        let result = check_certificate_in_crl(&cert_serial, &invalid_crl);
        assert!(result.is_err());
    }
}
