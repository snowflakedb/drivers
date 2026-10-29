use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD};
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::sign::Verifier;

use crate::common::mocks::auth;
use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;

#[test]
fn should_sign_jwt_with_the_algorithm_matching_the_ecdsa_key_curve() {
    for (nid, alg, digest) in [
        (Nid::X9_62_PRIME256V1, "ES256", MessageDigest::sha256()),
        (Nid::SECP384R1, "ES384", MessageDigest::sha384()),
        (Nid::SECP521R1, "ES512", MessageDigest::sha512()),
    ] {
        //Given Authentication is set to JWT and an ECDSA private key file is provided
        let group = EcGroup::from_curve_name(nid).unwrap();
        let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let pem = String::from_utf8(key.private_key_to_pem_pkcs8().unwrap()).unwrap();
        let mock = MockServerWithTls::start();
        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let key_file = private_key_helper::private_key_file_from_pem(&pem).unwrap();
        client.set_connection_option("private_key_file", key_file.path().to_str().unwrap());
        client.set_temp_key_file(key_file);
        mock.mount(auth::login_success_jwt());

        //When Trying to Connect
        client
            .connect()
            .unwrap_or_else(|e| panic!("{alg}: connect failed: {e}"));

        //Then The login request carries a JWT signed with the curve's algorithm that verifies under the public key
        assert_login_jwt_verifies(&mock, &key, alg, digest);
    }
}

fn assert_login_jwt_verifies(
    mock: &MockServerWithTls,
    key: &PKey<Private>,
    alg: &str,
    digest: MessageDigest,
) {
    let login = mock
        .received_requests()
        .into_iter()
        .find(|req| req.url.path() == "/session/v1/login-request")
        .expect("no login-request captured");
    let body: serde_json::Value = serde_json::from_slice(&login.body).unwrap();
    let token = body["data"]["TOKEN"].as_str().expect("login carries TOKEN");
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "{alg}: JWT must have three segments");

    let header: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
    assert_eq!(header["alg"], alg);

    let claims: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    let fingerprint = BASE64.encode(openssl::sha::sha256(&key.public_key_to_der().unwrap()));
    assert_eq!(claims["sub"], "TEST_ACCOUNT.TEST_USER");
    assert_eq!(
        claims["iss"],
        format!("TEST_ACCOUNT.TEST_USER.SHA256:{fingerprint}")
    );

    let raw = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let (r, s) = raw.split_at(raw.len() / 2);
    let der_signature = EcdsaSig::from_private_components(
        BigNum::from_slice(r).unwrap(),
        BigNum::from_slice(s).unwrap(),
    )
    .unwrap()
    .to_der()
    .unwrap();
    let mut verifier = Verifier::new(digest, key).unwrap();
    verifier
        .update(format!("{}.{}", parts[0], parts[1]).as_bytes())
        .unwrap();
    assert!(
        verifier.verify(&der_signature).unwrap(),
        "{alg}: JWT signature must verify under the EC public key"
    );
}

#[test]
fn should_fail_jwt_authentication_when_no_private_file_provided() {
    //Given Authentication is set to JWT
    let client = SnowflakeTestClient::with_int_tests_params(None);
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");

    //When Trying to Connect with no private file provided
    let result = client.connect();

    //Then There is error returned
    client.assert_missing_parameter_error(result);
}

#[test]
fn should_surface_jwt_credential_rejection_code() {
    //Given Authentication is set to JWT and the backend is configured to reject the JWT as invalid
    let mock = MockServerWithTls::start();
    let mut client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    let temp_key_file =
        private_key_helper::get_test_private_key_file().expect("Failed to create private key file");
    client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
    client.set_temp_key_file(temp_key_file);
    mock.mount(auth::login_failure_jwt_token_invalid());

    //When Trying to Connect
    let result = client.connect();

    //Then the raw GS code surfaces in the error
    let error = result.expect_err("Expected JWT authentication to fail");
    assert!(
        error.contains("390144"),
        "Expected the raw GS code 390144 (JWT_TOKEN_INVALID) in the error, not a \
         generic/hardcoded login-failure code — got: {error}"
    );
}
