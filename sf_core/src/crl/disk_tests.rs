#[cfg(test)]
mod disk_crl_tests {
    use crate::crl::certificate_parser::{
        check_certificate_in_crl, extract_crl_distribution_points, get_certificate_serial_number,
    };
    use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
    use crate::crl::validator::CrlValidator;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    use x509_parser::prelude::FromDer;

    /// Test parsing a real CRL from a local fixture on disk
    #[test]
    fn test_parse_real_crl_from_fixture() {
        // Read a real CRL from a local test fixture for testing
        // Place a valid DER-encoded CRL file at tests/fixtures/test.crl
        let crl_fixture_path = "tests/fixtures/test.crl";
        let crl_data_from_disk = match fs::read(crl_fixture_path) {
            Ok(data) => data,
            Err(_) => {
                // Skip test if fixture is missing
                eprintln!("CRL fixture file not found: {crl_fixture_path}");
                return;
            }
        };

        // Parse CRL using x509-parser
        let parse_result =
            x509_parser::revocation_list::CertificateRevocationList::from_der(&crl_data_from_disk);
        assert!(parse_result.is_ok(), "Failed to parse CRL from disk");

        let (_, crl) = parse_result.unwrap();

        // Verify CRL structure
        assert!(
            !crl.tbs_cert_list.issuer.as_raw().is_empty(),
            "CRL should have an issuer"
        );

        // Test certificate checking (allow CRLExpired due to stale public CRLs)
        let mock_serial = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let is_revoked = check_certificate_in_crl(&mock_serial, &crl_data_from_disk);
        match is_revoked {
            Ok(_v) => { /* acceptable */ }
            Err(crate::crl::error::CrlError::CrlExpired { .. }) => { /* acceptable */ }
            Err(e) => panic!("Certificate checking failed: {e}"),
        }

        // Quiet test: assert basic invariants instead of printing
        assert!(!crl_data_from_disk.is_empty());
        let _ = crl.iter_revoked_certificates().count();
        match is_revoked {
            Ok(_v) => { /* ok */ }
            Err(crate::crl::error::CrlError::CrlExpired { .. }) => { /* ok */ }
            Err(e) => panic!("Certificate checking failed: {e}"),
        }
    }

    /// Test certificate parsing functions with invalid data
    #[test]
    fn test_certificate_parsing_error_handling() {
        let invalid_cert_data = vec![0x30, 0x82, 0x01, 0x00]; // Incomplete DER

        // Test CRL distribution point extraction
        let result = extract_crl_distribution_points(&invalid_cert_data);
        assert!(result.is_err(), "Should fail with invalid certificate data");

        // Test serial number extraction
        let result = get_certificate_serial_number(&invalid_cert_data);
        assert!(result.is_err(), "Should fail with invalid certificate data");
    }

    /// Test CRL parsing with invalid data
    #[test]
    fn test_crl_parsing_error_handling() {
        let invalid_crl_data = vec![0x30, 0x82, 0x01, 0x00]; // Incomplete DER
        let mock_serial = vec![0x01, 0x02, 0x03];

        let result = check_certificate_in_crl(&mock_serial, &invalid_crl_data);
        assert!(result.is_err(), "Should fail with invalid CRL data");
    }

    /// Test that we're using the correct x509-parser library
    #[test]
    fn test_x509_parser_library_usage() {
        // Verify we can create basic X.509 structures
        use x509_parser::oid_registry::OID_X509_EXT_CRL_DISTRIBUTION_POINTS;

        // This OID should be available from the x509-parser crate
        assert_eq!(
            OID_X509_EXT_CRL_DISTRIBUTION_POINTS.to_string(),
            "2.5.29.31"
        );

        let _ = format!("{}", OID_X509_EXT_CRL_DISTRIBUTION_POINTS);
    }

    /// Test creating a minimal valid DER certificate for testing
    #[test]
    fn test_minimal_certificate_creation() {
        // This test verifies our understanding of DER format
        // A minimal X.509 certificate starts with SEQUENCE (0x30)
        let minimal_cert_start = vec![0x30, 0x82]; // SEQUENCE with length > 127 bytes

        // This should fail parsing (as expected) but not crash
        let result = extract_crl_distribution_points(&minimal_cert_start);
        assert!(result.is_err(), "Minimal certificate should fail parsing");

        // Quiet test
    }

    /// Benchmark CRL parsing performance
    #[tokio::test]
    async fn test_crl_parsing_performance() {
        use std::time::Instant;

        // Download a real CRL for performance testing
        let client = reqwest::Client::new();
        let crl_url = "http://crl.microsoft.com/pki/mscorp/crl/msitwww2.crl";

        if let Ok(response) = client.get(crl_url).send().await
            && let Ok(crl_data) = response.bytes().await
        {
            let crl_bytes = crl_data.to_vec();

            // Time the parsing operation
            let start = Instant::now();
            let parse_result =
                x509_parser::revocation_list::CertificateRevocationList::from_der(&crl_bytes);
            let parse_duration = start.elapsed();

            if let Ok((_, crl)) = parse_result {
                // Time the certificate checking operation
                let mock_serial = vec![0x01, 0x02, 0x03, 0x04, 0x05];
                let start = Instant::now();
                let _check_result = check_certificate_in_crl(&mock_serial, &crl_bytes);
                let check_duration = start.elapsed();

                let _ = (
                    crl_bytes.len(),
                    parse_duration,
                    check_duration,
                    crl.iter_revoked_certificates().count(),
                );

                // Performance assertions (reasonable limits)
                assert!(
                    parse_duration.as_millis() < 100,
                    "CRL parsing should be fast (< 100ms)"
                );
                assert!(
                    check_duration.as_millis() < 50,
                    "Certificate checking should be fast (< 50ms)"
                );
            }
        }
    }

    /// Test disk cache read path and that half-life refresh path does not panic
    #[tokio::test]
    async fn test_disk_cache_read_and_half_life_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        // Write a tiny fake CRL file to disk cache
        let url = "http://example.com/fake.crl";
        let fname = crate::crl::CrlCache::url_digest(url);
        let path = cache_dir.join(fname);
        let mut f = std::fs::File::create(&path).unwrap();
        let _ = f.write_all(&[0u8; 16]); // invalid CRL bytes, should not panic

        let cfg = CrlConfig {
            enable_disk_caching: true,
            cache_dir: Some(cache_dir),
            check_mode: CertRevocationCheckMode::Advisory,
            ..Default::default()
        };

        let validator = CrlValidator::new(cfg).unwrap();

        // Should not panic even with invalid cached bytes; will attempt network if needed
        let _ = validator.fetch_crl_with_cache(url).await;
    }

    /// Test atomic write helper does not panic and replaces file
    #[tokio::test]
    async fn test_atomic_write_helper() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("atomic.crl");
        fs::write(&path, b"old").unwrap();

        let cfg = CrlConfig {
            enable_disk_caching: true,
            ..Default::default()
        };
        let validator = CrlValidator::new(cfg).unwrap();

        // Call the helper indirectly via fetch path to avoid private method access
        // Simulate network fetch by writing file directly
        validator.write_crl_atomic(&path, b"new");
        let content = fs::read(&path).unwrap();
        assert_eq!(&content, b"new");
    }
}
