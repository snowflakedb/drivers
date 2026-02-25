use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use base64::{Engine as _, engine::general_purpose};
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use std::fs;
use tempfile::TempDir;

#[test]
fn should_authenticate_using_private_file_with_password() {
    //Given Authentication is set to JWT and private file with password is provided
    let client = SnowflakeTestClient::with_default_jwt_auth_params();

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_fail_jwt_authentication_when_invalid_private_key_provided() {
    //Given Authentication is set to JWT and invalid private key file is provided
    let mut client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    set_invalid_private_key_file(&mut client);

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}

#[test]
fn should_authenticate_using_private_key_as_bytes() {
    //Given Authentication is set to JWT and private key is provided as bytes
    let mut client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");

    let temp_key_file = private_key_helper::get_private_key_from_parameters(&client.parameters)
        .expect("Failed to create private key file");

    // Read the PEM private key file and convert to DER bytes
    let pem_contents = fs::read(temp_key_file.path()).expect("Failed to read private key file");
    let rsa = if let Some(password) = &client.parameters.private_key_password {
        Rsa::private_key_from_pem_passphrase(&pem_contents, password.as_bytes())
            .expect("Failed to parse encrypted PEM private key")
    } else {
        Rsa::private_key_from_pem(&pem_contents).expect("Failed to parse PEM private key")
    };
    let pkey = PKey::from_rsa(rsa).expect("Failed to create PKey from RSA");
    let der_bytes = pkey
        .private_key_to_der()
        .expect("Failed to convert private key to DER");

    // Send raw DER bytes via set_connection_option_bytes
    client.set_connection_option_bytes("private_key", &der_bytes);
    client.set_temp_key_file(temp_key_file);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_using_private_key_as_base64_string() {
    //Given Authentication is set to JWT and private key is provided as base64-encoded string
    let mut client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");

    let temp_key_file = private_key_helper::get_private_key_from_parameters(&client.parameters)
        .expect("Failed to create private key file");

    // Read the private key file and encode as base64
    let key_contents = fs::read(temp_key_file.path()).expect("Failed to read private key file");
    let key_base64 = general_purpose::STANDARD.encode(&key_contents);

    client.set_connection_option("private_key", &key_base64);
    if let Some(password) = &client.parameters.private_key_password {
        client.set_connection_option("private_key_password", password);
    }
    client.set_temp_key_file(temp_key_file);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_automatically_update_authenticator_to_jwt_if_key_pair_params_present() {
    //Given private key or private key file is provided and authenticator is not explicitly set
    let mut client = SnowflakeTestClient::with_default_params();
    // Note: NOT setting authenticator to SNOWFLAKE_JWT - it should be auto-detected

    let temp_key_file = private_key_helper::get_private_key_from_parameters(&client.parameters)
        .expect("Failed to create private key file");
    client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
    if let Some(password) = &client.parameters.private_key_password {
        client.set_connection_option("private_key_password", password);
    }
    client.set_temp_key_file(temp_key_file);

    //When Trying to Connect
    let result = client.connect();

    //Then Connector changes authenticator to JWT and login is successful and simple query can be executed
    client.verify_simple_query(result);
}

#[test]
fn should_authenticate_using_unencrypted_private_key_file() {
    //Given Authentication is set to JWT and an unencrypted private key file is provided (no password)
    let mut client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");

    let temp_key_file = private_key_helper::get_private_key_from_parameters(&client.parameters)
        .expect("Failed to create private key file");

    // Read encrypted PEM and decrypt it to produce an unencrypted key
    let pem_contents = fs::read(temp_key_file.path()).expect("Failed to read private key file");
    let password = client
        .parameters
        .private_key_password
        .as_ref()
        .expect("No private key password configured; cannot create unencrypted key for test");

    let rsa = Rsa::private_key_from_pem_passphrase(&pem_contents, password.as_bytes())
        .expect("Failed to decrypt private key");
    let pkey = PKey::from_rsa(rsa).expect("Failed to create PKey from RSA");
    let unencrypted_pem = pkey
        .private_key_to_pem_pkcs8()
        .expect("Failed to convert to unencrypted PEM");

    // Write unencrypted PEM to a temp file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let unencrypted_path = temp_dir.path().join("unencrypted_key.p8");
    fs::write(&unencrypted_path, &unencrypted_pem).expect("Failed to write unencrypted key");

    // Connect using unencrypted key file — no password needed
    client.set_connection_option("private_key_file", unencrypted_path.to_str().unwrap());
    client.set_temp_key_file(temp_key_file);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    client.verify_simple_query(result);
}

fn set_invalid_private_key_file(client: &mut SnowflakeTestClient) {
    let temp_key_file = private_key_helper::get_test_private_key_file()
        .expect("Failed to create test private key file");
    client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
    client.set_temp_key_file(temp_key_file);
}
