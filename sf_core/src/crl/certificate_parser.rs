use crate::crl::error::{CrlError, CrlParsingSnafu};
use chrono::TimeZone;
use snafu::Location;
use snafu::ResultExt;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::*;

/// Extract CRL distribution points from a DER-encoded certificate
pub fn extract_crl_distribution_points(cert_der: &[u8]) -> Result<Vec<String>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;

    let crl_urls: Vec<String> = cert
        .extensions()
        .iter()
        .filter(|ext| ext.oid == x509_parser::oid_registry::OID_X509_EXT_CRL_DISTRIBUTION_POINTS)
        .filter_map(|ext| match ext.parsed_extension() {
            ParsedExtension::CRLDistributionPoints(points) => Some(points.points.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|point| point.distribution_point.as_ref())
        .filter_map(|name| match name {
            x509_parser::extensions::DistributionPointName::FullName(names) => Some(names.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|general_name| match general_name {
            GeneralName::URI(uri) => Some(uri.to_string()),
            _ => None,
        })
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        .collect();

    if crl_urls.is_empty() {
        tracing::debug!("No CRL distribution points found in certificate");
    } else {
        let count = crl_urls.len();
        tracing::debug!("Found {count} CRL distribution points: {crl_urls:?}");
    }

    Ok(crl_urls)
}

/// Check if a certificate is short-lived using configured threshold (default 10 days)
pub fn is_short_lived_certificate_with_threshold(
    cert_der: &[u8],
    threshold_days: i64,
) -> Result<bool, CrlError> {
    let validity_inclusive = get_certificate_validity_duration_inclusive(cert_der)?;
    let validity_period_seconds = validity_inclusive.num_seconds();
    let threshold_seconds = threshold_days * 24 * 60 * 60;

    let is_short_lived = validity_period_seconds <= threshold_seconds;

    if is_short_lived {
        let days = validity_period_seconds / (24 * 60 * 60);
        tracing::info!("Certificate is short-lived ({days} days), skipping CRL check");
    }

    Ok(is_short_lived)
}

/// Use CA/B BR short-lived threshold: 10 days until 2026-03-15, then 7 days
/// Determined based on the certificate's notBefore date (issuance)
pub fn is_short_lived_certificate(cert_der: &[u8]) -> Result<bool, CrlError> {
    // CA/B BR transition date
    let cutoff = chrono::Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();

    let not_before_dt = get_certificate_not_before(cert_der)?;

    let threshold_days = if not_before_dt < cutoff { 10 } else { 7 };
    is_short_lived_certificate_with_threshold(cert_der, threshold_days)
}

/// Get certificate serial number as bytes for CRL comparison
pub fn get_certificate_serial_number(cert_der: &[u8]) -> Result<Vec<u8>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    Ok(cert.serial.to_bytes_be())
}

/// Get the certificate's notBefore timestamp as chrono::DateTime<Utc>
pub fn get_certificate_not_before(
    cert_der: &[u8],
) -> Result<chrono::DateTime<chrono::Utc>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    asn1_time_to_datetime(&cert.validity.not_before).ok_or_else(|| CrlError::CrlParsing {
        source: x509_parser::nom::Err::Failure(x509_parser::error::X509Error::InvalidDate),
        location: Location::new(file!(), line!(), 0),
    })
}

/// Get the certificate's notAfter timestamp as chrono::DateTime<Utc>
pub fn get_certificate_not_after(
    cert_der: &[u8],
) -> Result<chrono::DateTime<chrono::Utc>, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    asn1_time_to_datetime(&cert.validity.not_after).ok_or_else(|| CrlError::CrlParsing {
        source: x509_parser::nom::Err::Failure(x509_parser::error::X509Error::InvalidDate),
        location: Location::new(file!(), line!(), 0),
    })
}

/// Get (notBefore, notAfter) as chrono::DateTime<Utc>
pub fn get_certificate_validity(
    cert_der: &[u8],
) -> Result<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>), CrlError> {
    let not_before = get_certificate_not_before(cert_der)?;
    let not_after = get_certificate_not_after(cert_der)?;
    Ok((not_before, not_after))
}

/// Get the inclusive certificate validity duration (RFC 5280 §4.1.2.5)
pub fn get_certificate_validity_duration_inclusive(
    cert_der: &[u8],
) -> Result<chrono::Duration, CrlError> {
    let (not_before, not_after) = get_certificate_validity(cert_der)?;
    Ok((not_after + chrono::Duration::seconds(1)) - not_before)
}

/// Determine if a certificate is a CA certificate (BasicConstraints CA=true)
pub fn is_ca_certificate(cert_der: &[u8]) -> Result<bool, CrlError> {
    let (_, cert) =
        x509_parser::certificate::X509Certificate::from_der(cert_der).context(CrlParsingSnafu)?;
    for ext in cert.extensions() {
        if let ParsedExtension::BasicConstraints(bc) = ext.parsed_extension() {
            return Ok(bc.ca);
        }
    }
    // If BasicConstraints missing, treat as end-entity per RFC 5280 Section 4.2.1.9
    Ok(false)
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

    // Enforce IDP scope basics (attribute-only rejection). CA/user scoping is enforced by caller when both cert and CRL are known
    if let Ok(Some(idp)) = crate::tls::x509_utils::extract_crl_idp_scope(crl_der)
        && idp.only_attribute
    {
        return Ok(false);
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

    /// Generate a real self-signed certificate with a 5-day validity
    /// and assert short-lived detection works with default and custom thresholds
    #[test]
    fn test_short_lived_certificate_logic() {
        use openssl::asn1::Asn1Time;
        use openssl::hash::MessageDigest;
        use openssl::pkey::PKey;
        use openssl::rsa::Rsa;
        use openssl::x509::{X509, X509NameBuilder};

        // Generate keypair
        let rsa = Rsa::generate(2048).expect("rsa");
        let pkey = PKey::from_rsa(rsa).expect("pkey");

        // Subject/issuer
        let mut name_builder = X509NameBuilder::new().expect("name");
        name_builder
            .append_entry_by_text("CN", "testcn")
            .expect("cn");
        let name = name_builder.build();

        // Build self-signed cert valid for 5 days
        let mut builder = X509::builder().expect("builder");
        builder.set_version(2).expect("ver");
        builder.set_subject_name(&name).expect("subj");
        builder.set_issuer_name(&name).expect("iss");
        builder.set_pubkey(&pkey).expect("pub");
        let not_before = Asn1Time::days_from_now(0).expect("nb");
        let not_after = Asn1Time::days_from_now(5).expect("na");
        builder.set_not_before(&not_before).expect("set nb");
        builder.set_not_after(&not_after).expect("set na");
        builder.sign(&pkey, MessageDigest::sha256()).expect("sign");
        let cert = builder.build();
        let cert_der = cert.to_der().expect("der");

        // Default policy (10 days until 2026-03-15, then 7 days) should treat 5 days as short-lived
        assert!(
            is_short_lived_certificate(&cert_der).unwrap(),
            "Certificate with 5-day validity should be short-lived"
        );

        // Threshold 6: still short-lived
        assert!(
            is_short_lived_certificate_with_threshold(&cert_der, 6).unwrap(),
            "Should be short-lived with a 6-day threshold"
        );

        // Threshold 4: not short-lived
        assert!(
            !is_short_lived_certificate_with_threshold(&cert_der, 4).unwrap(),
            "Should not be short-lived with a 4-day threshold"
        );
    }
}
