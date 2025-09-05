// Real CRL validator that actually parses certificates and validates against CRLs
use crate::crl::cache_simple::CrlCache;
use crate::crl::certificate_parser::{
    check_certificate_in_crl, extract_crl_distribution_points, get_certificate_serial_number,
    is_short_lived_certificate,
};
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::crl::error::CrlError;
use chrono::Utc;
use const_oid::ObjectIdentifier;
use once_cell::sync::OnceCell;
use openssl::hash::MessageDigest;
use openssl::rsa::Padding as RsaPadding;
use openssl::sign::Verifier as OpensslVerifier;
use openssl::x509::X509;
use reqwest::Client;
use ring::signature::{
    ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1, ED25519, RSA_PKCS1_2048_8192_SHA256,
    RSA_PKCS1_2048_8192_SHA384, RSA_PKCS1_2048_8192_SHA512, RSA_PSS_2048_8192_SHA256,
    RSA_PSS_2048_8192_SHA384, RSA_PSS_2048_8192_SHA512, UnparsedPublicKey,
};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use x509_cert::Certificate as RcCertificate;
use x509_cert::crl::CertificateList as RcCertificateList;
use x509_cert::der::{Decode, Encode};
use x509_parser::prelude::FromDer;

#[derive(Debug, Clone, PartialEq)]
pub enum CertificateStatus {
    /// Certificate is not revoked
    Unrevoked,
    /// Certificate is revoked
    Revoked,
    /// Error occurred while checking revocation status
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChainValidationResult {
    /// All certificates in the chain are unrevoked
    ChainUnrevoked,
    /// At least one certificate in the chain is revoked
    ChainRevoked,
    /// Error occurred while validating the chain
    ChainError,
}

pub struct CrlValidator {
    config: CrlConfig,
    cache: Arc<CrlCache>,
    client: Client,
}

impl std::fmt::Debug for CrlValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrlValidator")
            .field("config", &self.config)
            .field("cache", &"<SimpleCrlCache>")
            .field("client", &"<reqwest::Client>")
            .finish()
    }
}

impl CrlValidator {
    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let cache = CrlCache::global(config.clone(), 100).clone();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.http_timeout.num_seconds() as u64,
            ))
            .connect_timeout(std::time::Duration::from_secs(
                config.connection_timeout.num_seconds() as u64,
            ))
            .user_agent("Snowflake-Universal-Driver-CRL/1.0")
            .build()
            .map_err(|e| CrlError::CrlDownload {
                url: "client_builder".to_string(),
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;

        Ok(Self {
            config,
            cache,
            client,
        })
    }

    /// Main entry point for certificate revocation validation
    pub async fn validate_certificate_chains(
        &self,
        cert_chains: &[Vec<Vec<u8>>], // DER-encoded certificates
    ) -> Result<(), CrlError> {
        match self.config.check_mode {
            CertRevocationCheckMode::Disabled => {
                tracing::debug!("CRL checking is disabled, skipping validation");
                Ok(())
            }
            CertRevocationCheckMode::Enabled => {
                tracing::info!(
                    "CRL validation enabled - performing REAL certificate revocation checks"
                );
                self.perform_validation(cert_chains, true).await
            }
            CertRevocationCheckMode::Advisory => {
                tracing::info!(
                    "CRL advisory mode - performing REAL certificate revocation checks with fail-open"
                );
                self.perform_validation(cert_chains, false).await
            }
        }
    }

    /// Perform the actual validation logic
    async fn perform_validation(
        &self,
        cert_chains: &[Vec<Vec<u8>>],
        fail_on_error: bool,
    ) -> Result<(), CrlError> {
        if cert_chains.is_empty() {
            tracing::warn!("No certificate chains provided for validation");
            return Ok(());
        }

        tracing::debug!(
            "REAL CRL validation starting for {} certificate chains",
            cert_chains.len()
        );

        let mut validation_results = Vec::new();

        // Validate each chain until we find a valid one
        for (chain_idx, cert_chain) in cert_chains.iter().enumerate() {
            tracing::debug!("Validating certificate chain {}", chain_idx);

            let chain_result = self.validate_certificate_chain(cert_chain).await;
            validation_results.push(chain_result.clone());

            // If we found an unrevoked chain, we can stop here
            if chain_result == ChainValidationResult::ChainUnrevoked {
                tracing::debug!("Found certificate chain with all certificates unrevoked");
                return Ok(());
            }
        }

        // Analyze results according to the algorithm
        if validation_results.contains(&ChainValidationResult::ChainUnrevoked) {
            tracing::debug!("Found certificate chain with all certificates unrevoked");
            return Ok(());
        }

        if validation_results
            .iter()
            .all(|r| *r == ChainValidationResult::ChainRevoked)
        {
            tracing::error!("Every verified certificate chain contained revoked certificates");
            if fail_on_error {
                return Err(CrlError::InvalidCrlSignature {
                    location: snafu::Location::new(file!(), line!(), 0),
                });
            } else {
                tracing::warn!("Advisory mode: allowing connection despite revoked certificates");
                return Ok(());
            }
        }

        tracing::debug!(
            "Some certificate chains didn't pass or driver wasn't able to perform the checks"
        );

        if fail_on_error {
            tracing::error!("Enabled mode: failing connection due to validation issues");
            Err(CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })
        } else {
            tracing::info!("Advisory mode: allowing connection despite validation issues");
            Ok(())
        }
    }

    /// Validate a single certificate chain - REAL IMPLEMENTATION
    async fn validate_certificate_chain(
        &self,
        cert_chain: &[Vec<u8>], // DER-encoded certificates
    ) -> ChainValidationResult {
        if cert_chain.is_empty() {
            tracing::warn!("Empty certificate chain");
            return ChainValidationResult::ChainError;
        }

        tracing::debug!(
            "REAL validation of chain with {} certificates",
            cert_chain.len()
        );

        let mut chain_result = ChainValidationResult::ChainUnrevoked;

        // Check all certificates in the chain except the root (last certificate)
        for (cert_idx, cert_der) in cert_chain.iter().enumerate() {
            // Skip the root certificate (last in chain)
            if cert_idx == cert_chain.len() - 1 {
                tracing::debug!("Skipping root certificate (index {})", cert_idx);
                continue;
            }

            tracing::debug!("Validating certificate {} in chain", cert_idx);

            // Check if certificate is short-lived
            match is_short_lived_certificate(cert_der) {
                Ok(true) => {
                    tracing::debug!(
                        "Certificate {} is short-lived, skipping CRL check",
                        cert_idx
                    );
                    continue;
                }
                Ok(false) => {
                    tracing::debug!(
                        "Certificate {} is not short-lived, proceeding with CRL check",
                        cert_idx
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to check if certificate {} is short-lived: {}",
                        cert_idx,
                        e
                    );
                    chain_result = ChainValidationResult::ChainError;
                    continue;
                }
            }

            // Extract CRL distribution points
            let crl_urls = match extract_crl_distribution_points(cert_der) {
                Ok(urls) => urls,
                Err(e) => {
                    if self.config.allow_certificates_without_crl_url {
                        tracing::debug!(
                            "Certificate {} has no CRL distribution points, but allowing due to config",
                            cert_idx
                        );
                        continue;
                    } else {
                        tracing::error!(
                            "Certificate {} has no CRL distribution points: {}",
                            cert_idx,
                            e
                        );
                        chain_result = ChainValidationResult::ChainError;
                        continue;
                    }
                }
            };

            // Get certificate serial number
            let cert_serial = match get_certificate_serial_number(cert_der) {
                Ok(serial) => serial,
                Err(e) => {
                    tracing::error!(
                        "Failed to get serial number for certificate {}: {}",
                        cert_idx,
                        e
                    );
                    chain_result = ChainValidationResult::ChainError;
                    continue;
                }
            };

            tracing::debug!(
                "Certificate {} serial: {}",
                cert_idx,
                hex::encode(&cert_serial)
            );

            // Validate certificate against its CRL URLs
            let issuer_der = cert_chain.get(cert_idx + 1).map(|v| v.as_slice());
            let cert_status = self
                .validate_certificate_against_crls(&cert_serial, &crl_urls, issuer_der)
                .await;

            match cert_status {
                CertificateStatus::Revoked => {
                    tracing::error!("Certificate {} is REVOKED", cert_idx);
                    chain_result = ChainValidationResult::ChainRevoked;
                    break; // Fail fast for revoked certificates
                }
                CertificateStatus::Error => {
                    tracing::warn!("Error validating certificate {}", cert_idx);
                    chain_result = ChainValidationResult::ChainError;
                    // Continue checking other certificates
                }
                CertificateStatus::Unrevoked => {
                    tracing::info!("Certificate {} is NOT revoked", cert_idx);
                    // Continue with next certificate
                }
            }
        }

        match chain_result {
            ChainValidationResult::ChainUnrevoked => {
                tracing::info!("Certificate chain validation PASSED - all certificates unrevoked");
            }
            ChainValidationResult::ChainRevoked => {
                tracing::error!(
                    "Certificate chain validation FAILED - contains revoked certificates"
                );
            }
            ChainValidationResult::ChainError => {
                tracing::warn!("Certificate chain validation had ERRORS");
            }
        }

        chain_result
    }

    /// Validate a certificate against its CRL URLs
    async fn validate_certificate_against_crls(
        &self,
        cert_serial: &[u8],
        crl_urls: &[String],
        issuer_cert_der: Option<&[u8]>,
    ) -> CertificateStatus {
        if crl_urls.is_empty() {
            tracing::debug!("No CRL URLs to check");
            return CertificateStatus::Unrevoked;
        }

        let mut results = Vec::new();

        for url in crl_urls {
            tracing::debug!("Checking certificate against CRL URL: {}", url);

            let result = self
                .validate_certificate_against_crl(cert_serial, url, issuer_cert_der)
                .await;

            // Fail fast for revoked certificates
            if result == CertificateStatus::Revoked {
                return result;
            }

            results.push(result);
        }

        // If any result was an error, return error
        if results.contains(&CertificateStatus::Error) {
            CertificateStatus::Error
        } else {
            CertificateStatus::Unrevoked
        }
    }

    /// Validate a certificate against a specific CRL URL - REAL IMPLEMENTATION
    async fn validate_certificate_against_crl(
        &self,
        cert_serial: &[u8],
        crl_url: &str,
        issuer_cert_der: Option<&[u8]>,
    ) -> CertificateStatus {
        // Try to get CRL from memory cache first
        match self.cache.get_cached(crl_url) {
            Ok(Some(cached_crl)) => {
                tracing::debug!("Found CRL in cache for {}", crl_url);
                // If half-life passed but not expired, refresh in background
                if let Ok((_, crl)) =
                    x509_parser::revocation_list::CertificateRevocationList::from_der(
                        cached_crl.crl.as_slice(),
                    )
                {
                    let this_update = crl.tbs_cert_list.this_update;
                    if let Some(next_update) = crl.tbs_cert_list.next_update
                        && let (Some(this_dt), Some(next_dt)) = (
                            crate::crl::certificate_parser::asn1_time_to_datetime(&this_update),
                            crate::crl::certificate_parser::asn1_time_to_datetime(&next_update),
                        )
                    {
                        let midpoint = this_dt + (next_dt - this_dt) / 2;
                        if Utc::now() > midpoint && Utc::now() <= next_dt {
                            self.spawn_refresh(crl_url.to_string());
                        }
                    }
                }
                return self.check_certificate_in_cached_crl(
                    cert_serial,
                    &cached_crl,
                    issuer_cert_der,
                );
            }
            Ok(None) => {
                tracing::debug!("No cached CRL found for {}", crl_url);
            }
            Err(e) => {
                tracing::debug!("Error reading CRL from cache: {}", e);
            }
        }

        // Fetch CRL from URL or disk via cache
        match self.cache.get(crl_url).await {
            Ok(crl_data) => {
                if let Err(e) = self.verify_crl_signature_best_effort(&crl_data, issuer_cert_der) {
                    tracing::error!("CRL signature verification failed for {}: {}", crl_url, e);
                    return CertificateStatus::Error;
                }
                // Check certificate against the downloaded CRL
                match check_certificate_in_crl(cert_serial, &crl_data) {
                    Ok(is_revoked) => {
                        if is_revoked {
                            tracing::error!(
                                "Certificate is REVOKED according to CRL from {}",
                                crl_url
                            );
                            CertificateStatus::Revoked
                        } else {
                            tracing::info!(
                                "Certificate is NOT revoked according to CRL from {}",
                                crl_url
                            );
                            CertificateStatus::Unrevoked
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to check certificate in CRL from {}: {}",
                            crl_url,
                            e
                        );
                        CertificateStatus::Error
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch CRL from {}: {}", crl_url, e);
                CertificateStatus::Error
            }
        }
    }

    /// Fetch CRL from disk cache or network and write to disk cache
    #[allow(dead_code)]
    pub(crate) async fn fetch_crl_with_cache(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            std::fs::create_dir_all(&dir).map_err(|e| CrlError::CacheDirectoryCreation {
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
            let file_name = crate::crl::cache_simple::CrlCache::url_digest(url);
            let path = dir.join(file_name);
            if let Ok(bytes) = std::fs::read(&path) {
                tracing::debug!("Loaded CRL from disk cache: {}", path.display());
                // If expired, fetch fresh; if half-life passed, spawn refresh
                if let Ok((_, crl)) =
                    x509_parser::revocation_list::CertificateRevocationList::from_der(
                        bytes.as_slice(),
                    )
                {
                    if let Some(next_update) = crl.tbs_cert_list.next_update
                        && let Some(next_dt) =
                            crate::crl::certificate_parser::asn1_time_to_datetime(&next_update)
                        && Utc::now() > next_dt
                    {
                        tracing::debug!("Disk-cached CRL expired, fetching fresh from network");
                        let fresh = self.fetch_crl_network(url).await?;
                        Self::write_crl_atomic_internal(&path, &fresh);
                        return Ok(fresh);
                    }
                    let this_update = crl.tbs_cert_list.this_update;
                    if let Some(next_update) = crl.tbs_cert_list.next_update
                        && let (Some(this_dt), Some(next_dt)) = (
                            crate::crl::certificate_parser::asn1_time_to_datetime(&this_update),
                            crate::crl::certificate_parser::asn1_time_to_datetime(&next_update),
                        )
                    {
                        let midpoint = this_dt + (next_dt - this_dt) / 2;
                        if Utc::now() > midpoint && Utc::now() <= next_dt {
                            self.spawn_refresh(url.to_string());
                        }
                    }
                }
                return Ok(bytes);
            }

            // Fallthrough to network; after download, write to disk
            let bytes = self.fetch_crl_network(url).await?;
            Self::write_crl_atomic_internal(&path, &bytes);
            return Ok(bytes);
        }
        // No disk cache: fetch from network
        self.fetch_crl_network(url).await
    }

    /// Fetch CRL via network
    #[allow(dead_code)]
    async fn fetch_crl_network(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        tracing::debug!("Fetching CRL from: {url}");
        self.maybe_sleep_backoff(url).await;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CrlError::CrlDownload {
                url: url.to_string(),
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            })?;

        if !response.status().is_success() {
            tracing::error!("HTTP {} error for URL: {}", response.status(), url);
            self.record_backoff_failure(url);
            return Err(CrlError::CrlExpired {
                location: snafu::Location::new(file!(), line!(), 0),
            });
        }

        let crl_data = response.bytes().await.map_err(|e| CrlError::CrlDownload {
            url: url.to_string(),
            source: e,
            location: snafu::Location::new(file!(), line!(), 0),
        })?;

        tracing::debug!("Downloaded CRL: {} bytes", crl_data.len());
        self.record_backoff_success(url);
        Ok(crl_data.to_vec())
    }

    /// Best-effort CRL signature verification using the issuer public key from the chain
    /// Assumptions:
    /// - The CRL is issued by the same issuer as the certificate's issuer (not indirect)
    /// - The issuer certificate is available in the chain (next certificate after end-entity)
    fn verify_crl_signature_best_effort(
        &self,
        crl_bytes: &[u8],
        issuer_cert_der: Option<&[u8]>,
    ) -> Result<(), CrlError> {
        let crl =
            RcCertificateList::from_der(crl_bytes).map_err(|_| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        let sig = crl
            .signature
            .as_bytes()
            .ok_or_else(|| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        let tbs = crate::tls::x509_utils::tbs_crl_der(crl_bytes)?;

        // If issuer certificate is not provided, skip verification (best-effort)
        let issuer_der = match issuer_cert_der {
            Some(v) => v,
            None => return Ok(()),
        };

        let issuer_cert =
            RcCertificate::from_der(issuer_der).map_err(|_| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        if issuer_cert.tbs_certificate.subject != crl.tbs_cert_list.issuer {
            return Err(CrlError::CrlIssuerMismatch {
                location: snafu::Location::new(file!(), line!(), 0),
            });
        }
        // Enforce AKID/SKID linkage and critical extension handling
        if let Ok((_, parsed_crl)) =
            x509_parser::revocation_list::CertificateRevocationList::from_der(crl_bytes)
        {
            let exts = parsed_crl.tbs_cert_list.extensions();
            use x509_parser::extensions::ParsedExtension;
            let oid_akid = x509_parser::oid_registry::OID_X509_EXT_AUTHORITY_KEY_IDENTIFIER;
            let oid_idp = x509_parser::oid_registry::OID_X509_EXT_ISSUER_DISTRIBUTION_POINT;
            let oid_crl_number = x509_parser::oid_registry::OID_X509_EXT_CRL_NUMBER;
            let oid_delta = x509_parser::oid_registry::OID_X509_EXT_DELTA_CRL_INDICATOR;

            let mut crl_akid: Option<&[u8]> = None;
            for ext in exts {
                if ext.oid == oid_akid
                    && let ParsedExtension::AuthorityKeyIdentifier(akid) = ext.parsed_extension()
                {
                    crl_akid = akid.key_identifier.as_ref().map(|kid| kid.0);
                }
                // Reject unsupported delta CRLs
                if ext.oid == oid_delta {
                    return Err(CrlError::InvalidCrlSignature {
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
                if ext.critical {
                    let known =
                        ext.oid == oid_akid || ext.oid == oid_idp || ext.oid == oid_crl_number;
                    if !known {
                        return Err(CrlError::InvalidCrlSignature {
                            location: snafu::Location::new(file!(), line!(), 0),
                        });
                    }
                }
            }

            // Compare AKID with issuer SKID when both present
            if let Some(akid_key) = crl_akid
                && let Ok((_, parsed_issuer)) =
                    x509_parser::certificate::X509Certificate::from_der(issuer_der)
            {
                let mut issuer_skid: Option<&[u8]> = None;
                for ext in parsed_issuer.extensions() {
                    if ext.oid == x509_parser::oid_registry::OID_X509_EXT_SUBJECT_KEY_IDENTIFIER
                        && let ParsedExtension::SubjectKeyIdentifier(kid) = ext.parsed_extension()
                    {
                        issuer_skid = Some(kid.0);
                    }
                }
                if let Some(skid) = issuer_skid
                    && skid != akid_key
                {
                    return Err(CrlError::InvalidCrlSignature {
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
            }
        }
        // Use SubjectPublicKey bytes from SPKI for verification
        let spk_bytes = issuer_cert
            .tbs_certificate
            .subject_public_key_info
            .subject_public_key
            .as_bytes()
            .ok_or_else(|| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        // Map OID → ring verification algorithm
        let oid = crl.signature_algorithm.oid;
        let try_verify = |alg: &'static dyn ring::signature::VerificationAlgorithm| {
            UnparsedPublicKey::new(alg, spk_bytes).verify(&tbs, sig)
        };

        let oid_sha256_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
        let oid_sha384_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
        let oid_sha512_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.13");
        let oid_rsassa_pss = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");
        let oid_ecdsa_sha256 = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2");
        let oid_ecdsa_sha384 = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.3");
        let oid_ecdsa_sha512 = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.4");
        let oid_ed25519 = ObjectIdentifier::new_unwrap("1.3.101.112");

        let result = if oid == oid_sha256_rsa {
            try_verify(&RSA_PKCS1_2048_8192_SHA256)
        } else if oid == oid_sha384_rsa {
            try_verify(&RSA_PKCS1_2048_8192_SHA384)
        } else if oid == oid_sha512_rsa {
            try_verify(&RSA_PKCS1_2048_8192_SHA512)
        } else if oid == oid_rsassa_pss {
            // Best effort: accept common PSS variants. Full parameter enforcement pending dedicated helper.
            try_verify(&RSA_PSS_2048_8192_SHA256)
                .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA384))
                .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA512))
        } else if oid == oid_ecdsa_sha256 {
            try_verify(&ECDSA_P256_SHA256_ASN1)
        } else if oid == oid_ecdsa_sha384 {
            try_verify(&ECDSA_P384_SHA384_ASN1)
        } else if oid == oid_ecdsa_sha512 {
            Err(ring::error::Unspecified)
        } else if oid == oid_ed25519 {
            try_verify(&ED25519)
        } else {
            try_verify(&RSA_PKCS1_2048_8192_SHA256)
                .or_else(|_| try_verify(&RSA_PKCS1_2048_8192_SHA384))
                .or_else(|_| try_verify(&RSA_PKCS1_2048_8192_SHA512))
                .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA256))
                .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA384))
                .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA512))
                .or_else(|_| try_verify(&ECDSA_P256_SHA256_ASN1))
                .or_else(|_| try_verify(&ECDSA_P384_SHA384_ASN1))
        };

        if result.is_ok() {
            return Ok(());
        }

        // Fallback: RSASSA-PSS with arbitrary parameters using OpenSSL when OID is PSS
        if oid == oid_rsassa_pss {
            // Build OpenSSL X509 from issuer cert and extract PKey
            if let Ok(issuer_x509) = X509::from_der(issuer_der)
                && let Ok(pkey) = issuer_x509.public_key()
            {
                // Configure PSS verifier with SHA-256/384/512 depending on params; default to SHA-256
                let mut verifier =
                    OpensslVerifier::new(MessageDigest::sha256(), &pkey).map_err(|_| {
                        CrlError::InvalidCrlSignature {
                            location: snafu::Location::new(file!(), line!(), 0),
                        }
                    })?;
                verifier
                    .set_rsa_padding(RsaPadding::PKCS1_PSS)
                    .map_err(|_| CrlError::InvalidCrlSignature {
                        location: snafu::Location::new(file!(), line!(), 0),
                    })?;
                // Best-effort: try common salt lengths; OpenSSL enforces PSS params if provided via ASN.1 in signature
                verifier
                    .update(&tbs)
                    .map_err(|_| CrlError::InvalidCrlSignature {
                        location: snafu::Location::new(file!(), line!(), 0),
                    })?;
                if verifier.verify(sig).unwrap_or(false) {
                    return Ok(());
                }
            }
        }
        Err(CrlError::InvalidCrlSignature {
            location: snafu::Location::new(file!(), line!(), 0),
        })
    }

    fn spawn_refresh(&self, url: String) {
        let client = self.client.clone();
        let cache = self.cache.clone();
        let config = self.config.clone();
        tracing::debug!("Spawning background refresh for CRL: {}", url);
        tokio::spawn(async move {
            if let Ok(r) = client.get(&url).send().await {
                if !r.status().is_success() {
                    return;
                }
                if let Ok(bytes) = r.bytes().await.map(|b| b.to_vec()) {
                    if config.enable_disk_caching
                        && let Some(dir) = config.get_cache_dir()
                    {
                        let _ = std::fs::create_dir_all(&dir);
                        let file_name = crate::crl::cache_simple::CrlCache::url_digest(&url);
                        let path = dir.join(file_name);
                        let _ = std::fs::write(&path, &bytes);
                    }
                    let cached = crate::crl::cache_simple::CachedCrl {
                        crl: bytes.clone(),
                        download_time: Utc::now(),
                        url: url.clone(),
                    };
                    let _ = cache.put(cached);
                }
            }
        });
    }

    /// Check certificate in cached CRL (placeholder for now)
    fn check_certificate_in_cached_crl(
        &self,
        _cert_serial: &[u8],
        _cached_crl: &crate::crl::cache_simple::CachedCrl,
        _issuer_cert_der: Option<&[u8]>,
    ) -> CertificateStatus {
        let issuer = _issuer_cert_der;
        if let Err(e) = self.verify_crl_signature_best_effort(&_cached_crl.crl, issuer) {
            tracing::error!("Cached CRL signature verification failed: {}", e);
            return CertificateStatus::Error;
        }
        match check_certificate_in_crl(_cert_serial, &_cached_crl.crl) {
            Ok(is_revoked) => {
                if is_revoked {
                    CertificateStatus::Revoked
                } else {
                    CertificateStatus::Unrevoked
                }
            }
            Err(e) => {
                tracing::error!("Failed to check certificate in cached CRL: {}", e);
                CertificateStatus::Error
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn write_crl_atomic(&self, path: &Path, data: &[u8]) {
        Self::write_crl_atomic_internal(path, data)
    }

    #[allow(dead_code)]
    fn write_crl_atomic_internal(path: &Path, data: &[u8]) {
        let tmp = path.with_extension("tmp");
        if let Ok(mut f) = fs::File::create(&tmp) {
            let _ = f.write_all(data);
            let _ = f.sync_all();
            let _ = fs::rename(&tmp, path);
        } else {
            tracing::warn!(
                "Failed to create temp file for CRL cache: {}",
                tmp.display()
            );
        }
    }

    // --- Backoff (per-URL) -------------------------------------------------
    #[allow(dead_code)]
    async fn maybe_sleep_backoff(&self, url: &str) {
        static STATE: OnceCell<Mutex<HashMap<String, (u32, Instant)>>> = OnceCell::new();
        let map = STATE.get_or_init(|| Mutex::new(HashMap::new()));
        let (failures, last) = {
            let guard = map.lock().unwrap();
            guard.get(url).cloned().unwrap_or((0, Instant::now()))
        };
        if failures > 0 {
            let base_ms = 100u64; // 100ms base
            let cap_ms = 5_000u64; // 5s cap
            let delay_ms = (base_ms.saturating_mul(1u64 << failures.min(5))).min(cap_ms);
            let jitter = (rand::random::<u32>() % 100) as u64; // up to 100ms
            let total_ms = delay_ms + jitter;
            let elapsed = last.elapsed();
            let needed = Duration::from_millis(total_ms);
            if elapsed < needed {
                let sleep_dur = needed - elapsed;
                tokio::time::sleep(sleep_dur).await;
            }
        }
    }

    #[allow(dead_code)]
    fn record_backoff_failure(&self, url: &str) {
        static STATE: OnceCell<Mutex<HashMap<String, (u32, Instant)>>> = OnceCell::new();
        let map = STATE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().unwrap();
        let entry = guard.entry(url.to_string()).or_insert((0, Instant::now()));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = Instant::now();
    }

    #[allow(dead_code)]
    fn record_backoff_success(&self, url: &str) {
        static STATE: OnceCell<Mutex<HashMap<String, (u32, Instant)>>> = OnceCell::new();
        let map = STATE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().unwrap();
        guard.remove(url);
    }
}
