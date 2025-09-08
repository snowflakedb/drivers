// Small helpers to centralize dual x509 crate usage
use crate::crl::error::CrlError;
use const_oid::ObjectIdentifier;
use x509_cert::crl::CertificateList as RcCertificateList;
use x509_cert::der::{Decode, Encode};
use x509_parser::prelude::FromDer;

/// Best-effort CRL signature verification using issuer public key
/// Returns Ok(()) if verification passes or issuer is None; Err otherwise.
pub fn verify_crl_signature_best_effort(
    crl_der: &[u8],
    issuer_der: Option<&[u8]>,
) -> Result<(), CrlError> {
    let crl = RcCertificateList::from_der(crl_der).map_err(|_| CrlError::InvalidCrlSignature {
        location: snafu::Location::new(file!(), line!(), 0),
    })?;
    let sig = crl
        .signature
        .as_bytes()
        .ok_or_else(|| CrlError::InvalidCrlSignature {
            location: snafu::Location::new(file!(), line!(), 0),
        })?;
    let tbs = tbs_crl_der(crl_der)?;

    let issuer_der = match issuer_der {
        Some(v) => v,
        None => return Ok(()),
    };
    let issuer_cert = x509_cert::Certificate::from_der(issuer_der).map_err(|_| {
        CrlError::InvalidCrlSignature {
            location: snafu::Location::new(file!(), line!(), 0),
        }
    })?;
    if issuer_cert.tbs_certificate.subject != crl.tbs_cert_list.issuer {
        return Err(CrlError::CrlIssuerMismatch {
            location: snafu::Location::new(file!(), line!(), 0),
        });
    }

    // Enforce AKID/SKID and critical extension policy
    if let Ok((_, parsed_crl)) =
        x509_parser::revocation_list::CertificateRevocationList::from_der(crl_der)
    {
        use x509_parser::extensions::ParsedExtension;
        let oid_akid = x509_parser::oid_registry::OID_X509_EXT_AUTHORITY_KEY_IDENTIFIER;
        let oid_idp = x509_parser::oid_registry::OID_X509_EXT_ISSUER_DISTRIBUTION_POINT;
        let oid_crl_number = x509_parser::oid_registry::OID_X509_EXT_CRL_NUMBER;
        let oid_delta = x509_parser::oid_registry::OID_X509_EXT_DELTA_CRL_INDICATOR;
        let mut crl_akid: Option<&[u8]> = None;
        for ext in parsed_crl.tbs_cert_list.extensions() {
            if ext.oid == oid_akid
                && let ParsedExtension::AuthorityKeyIdentifier(akid) = ext.parsed_extension()
            {
                crl_akid = akid.key_identifier.as_ref().map(|kid| kid.0);
            }
            if ext.oid == oid_delta {
                return Err(CrlError::InvalidCrlSignature {
                    location: snafu::Location::new(file!(), line!(), 0),
                });
            }
            if ext.critical {
                let known = ext.oid == oid_akid || ext.oid == oid_idp || ext.oid == oid_crl_number;
                if !known {
                    return Err(CrlError::InvalidCrlSignature {
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
            }
        }
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

    // Verify signature
    let spk_bytes = issuer_cert
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .as_bytes()
        .ok_or_else(|| CrlError::InvalidCrlSignature {
            location: snafu::Location::new(file!(), line!(), 0),
        })?;
    let try_verify = |alg: &'static dyn ring::signature::VerificationAlgorithm| {
        ring::signature::UnparsedPublicKey::new(alg, spk_bytes).verify(&tbs, sig)
    };
    use ring::signature::{
        ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1, ED25519, RSA_PKCS1_2048_8192_SHA256,
        RSA_PKCS1_2048_8192_SHA384, RSA_PKCS1_2048_8192_SHA512, RSA_PSS_2048_8192_SHA256,
        RSA_PSS_2048_8192_SHA384, RSA_PSS_2048_8192_SHA512,
    };
    let oid = crl.signature_algorithm.oid;
    let oid_sha256_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
    let oid_sha384_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
    let oid_sha512_rsa = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.13");
    let oid_rsassa_pss = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");
    let oid_ecdsa_sha256 = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2");
    let oid_ecdsa_sha384 = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.3");
    let oid_ed25519 = ObjectIdentifier::new_unwrap("1.3.101.112");

    let result = if oid == oid_sha256_rsa {
        try_verify(&RSA_PKCS1_2048_8192_SHA256)
    } else if oid == oid_sha384_rsa {
        try_verify(&RSA_PKCS1_2048_8192_SHA384)
    } else if oid == oid_sha512_rsa {
        try_verify(&RSA_PKCS1_2048_8192_SHA512)
    } else if oid == oid_rsassa_pss {
        try_verify(&RSA_PSS_2048_8192_SHA256)
            .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA384))
            .or_else(|_| try_verify(&RSA_PSS_2048_8192_SHA512))
    } else if oid == oid_ecdsa_sha256 {
        try_verify(&ECDSA_P256_SHA256_ASN1)
    } else if oid == oid_ecdsa_sha384 {
        try_verify(&ECDSA_P384_SHA384_ASN1)
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

    // OpenSSL fallback for RSASSA-PSS
    if oid == oid_rsassa_pss
        && let Ok(issuer_x509) = openssl::x509::X509::from_der(issuer_der)
        && let Ok(pkey) = issuer_x509.public_key()
    {
        let mut verifier =
            openssl::sign::Verifier::new(openssl::hash::MessageDigest::sha256(), &pkey).map_err(
                |_| CrlError::InvalidCrlSignature {
                    location: snafu::Location::new(file!(), line!(), 0),
                },
            )?;
        verifier
            .set_rsa_padding(openssl::rsa::Padding::PKCS1_PSS)
            .map_err(|_| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        verifier
            .update(&tbs)
            .map_err(|_| CrlError::InvalidCrlSignature {
                location: snafu::Location::new(file!(), line!(), 0),
            })?;
        if verifier.verify(sig).unwrap_or(false) {
            return Ok(());
        }
    }
    Err(CrlError::InvalidCrlSignature {
        location: snafu::Location::new(file!(), line!(), 0),
    })
}

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

/// Extract issuer SKID if present
pub fn extract_skid(issuer_der: &[u8]) -> Option<Vec<u8>> {
    if let Ok((_, issuer)) = x509_parser::certificate::X509Certificate::from_der(issuer_der) {
        for ext in issuer.extensions() {
            if ext.oid == x509_parser::oid_registry::OID_X509_EXT_SUBJECT_KEY_IDENTIFIER
                && let x509_parser::extensions::ParsedExtension::SubjectKeyIdentifier(k) =
                    ext.parsed_extension()
            {
                return Some(k.0.to_vec());
            }
        }
    }
    None
}

/// Stable hash of the issuer Subject DER (not its string form)
pub fn subject_der_hash(issuer_der: &[u8]) -> Option<Vec<u8>> {
    use x509_cert::der::Encode;
    let cert = x509_cert::Certificate::from_der(issuer_der).ok()?;
    let der = cert.tbs_certificate.subject.to_der().ok()?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(&der);
    Some(hasher.finalize().to_vec())
}
