use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::{create_test_file, decompress_gzipped_file};
use crate::common::private_key_helper::get_private_key_pem_from_parameters;
use crate::common::put_get_common::{assert_file_exists, get_file_from_stage, upload_to_stage};
use crate::common::snowflake_test_client::{SnowflakeTestClient, unwrap_single_rs_handle};
use openssl::pkey::{PKey, Private};
use openssl::symm::Cipher;
use sf_core::protobuf::generated::database_driver_v1::result_chunk;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn should_select_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And a default connection is logged in with CRL off
    let client = SnowflakeTestClient::connect_with_default_auth();

    // When SELECT 1 is executed
    let rows = select_one(&client);

    // Then the query returns 1
    assert_eq!(rows, vec![vec![1]]);
}

#[test]
fn should_select_with_crl_enabled_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And CRL is enabled before the connection is established
    let client = SnowflakeTestClient::with_default_jwt_auth_params();
    client.set_connection_option("crl_check_mode", "ENABLED");
    client.connect().expect("CRL-enabled login failed");

    // When SELECT 1 is executed
    let rows = select_one(&client);

    // Then the query returns 1
    assert_eq!(rows, vec![vec![1]]);
}

#[test]
fn should_put_and_get_on_a_fips_build() {
    // Given the linked AWS-LC module reports FIPS mode
    assert_fips_mode();

    // And a default connection is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let filename = "fips_smoke.txt";
    let content = "fips-smoke\n";
    let file_path = create_test_file(temp_dir.path(), filename, content);
    let stage_name = "FIPS_SMOKE";

    // When a small file is uploaded to a unique stage and downloaded
    upload_to_stage(
        &client,
        stage_name,
        file_path.to_str().expect("test file path is utf-8"),
    );
    let (_get_result, download_dir) = get_file_from_stage(&client, stage_name, filename);

    // Then the downloaded file matches the upload
    let gzipped_filename = format!("{filename}.gz");
    assert_file_exists(&download_dir, &gzipped_filename);
    let downloaded = decompress_gzipped_file(download_dir.path().join(&gzipped_filename))
        .expect("decompress downloaded file");
    assert_eq!(downloaded, content);
}

#[test]
fn should_negotiate_tls_with_fips_on_a_fips_build() {
    // Given both the linked provider and the build report FIPS TLS
    let status = sf_core::tls::tls_status();
    assert!(status.tls_provider_is_fips);
    assert!(status.fips_build_enabled);
    // And the driver has resolved the account's effective endpoint
    let client = SnowflakeTestClient::connect_with_default_auth();
    let server_url = client
        .connection_get_info_blocking(false)
        .expect("read the resolved connection endpoint")
        .server_url
        .expect("connected client has a server URL");
    let server_url = url::Url::parse(&server_url).expect("parse the resolved Snowflake URL");
    assert_eq!(server_url.scheme(), "https");
    let host = server_url.host_str().expect("Snowflake URL has a host");
    let port = server_url
        .port_or_known_default()
        .expect("Snowflake URL has an HTTPS port");
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
    // CryptoModule owns this provider too; it is deliberately not process-global.
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("FIPS provider supports the default TLS versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    assert!(config.fips());

    // When a certificate-verified TLS handshake reaches the configured account
    let server_name = rustls::pki_types::ServerName::try_from(host.to_owned())
        .expect("Snowflake host is a valid server name");
    let mut connection = rustls::ClientConnection::new(Arc::new(config), server_name)
        .expect("create TLS connection");
    let mut socket = TcpStream::connect((host, port)).expect("connect to Snowflake TLS endpoint");
    let timeout = Some(Duration::from_secs(30));
    socket
        .set_read_timeout(timeout)
        .expect("set TLS read timeout");
    socket
        .set_write_timeout(timeout)
        .expect("set TLS write timeout");
    connection
        .complete_io(&mut socket)
        .expect("complete Snowflake TLS handshake");

    // Then the negotiated suite is FIPS-approved and the protocol is TLS 1.2 or 1.3
    assert!(
        connection
            .negotiated_cipher_suite()
            .expect("TLS handshake negotiated a cipher suite")
            .fips()
    );
    assert!(matches!(
        connection.protocol_version(),
        Some(rustls::ProtocolVersion::TLSv1_2 | rustls::ProtocolVersion::TLSv1_3)
    ));
}

#[test]
fn should_login_with_aes_encrypted_private_key_on_a_fips_build() {
    // Given the account's private key is re-encrypted as AES-256-CBC PBES2 PKCS#8
    assert_fips_mode();
    let client = SnowflakeTestClient::with_default_params();
    let key = configured_private_key(&client);
    let passphrase = "fips-smoke-key-passphrase";
    let encrypted_key = key
        .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), passphrase.as_bytes())
        .expect("encrypt PKCS#8 key with AES-256-CBC");
    let _key_dir = configure_encrypted_key_file(&client, &encrypted_key, passphrase);

    // When key-pair authentication loads that file and its passphrase
    client
        .connect()
        .expect("AES-encrypted private-key login failed");

    // Then the authenticated connection can execute SELECT 1
    assert_eq!(select_one(&client), vec![vec![1]]);
}

#[test]
fn should_reject_triple_des_private_key_on_a_fips_build() {
    // Given the account's private key is re-encrypted as 3DES-CBC PBES2 PKCS#8
    assert_fips_mode();
    let client = SnowflakeTestClient::with_default_params();
    let key = configured_private_key(&client);
    let passphrase = "fips-smoke-key-passphrase";
    let encrypted_key = key
        .private_key_to_pem_pkcs8_passphrase(Cipher::des_ede3_cbc(), passphrase.as_bytes())
        .expect("encrypt PKCS#8 key with 3DES-CBC");
    let _key_dir = configure_encrypted_key_file(&client, &encrypted_key, passphrase);

    // When key-pair authentication tries to load the encrypted key
    let error = client.connect().expect_err("3DES key must be rejected");

    // Then the failure identifies the FIPS refusal and the supported conversion
    assert!(error.contains("3DES-CBC"), "{error}");
    assert!(error.contains("FIPS build"), "{error}");
    assert!(
        error.contains("openssl pkcs8 -topk8 -v2 aes-256-cbc"),
        "{error}"
    );
}

#[test]
fn should_reject_legacy_encrypted_private_key_on_a_fips_build() {
    // Given the account's private key uses traditional encrypted PEM's MD5 KDF
    assert_fips_mode();
    let client = SnowflakeTestClient::with_default_params();
    let key = configured_private_key(&client);
    let passphrase = "fips-smoke-key-passphrase";
    let encrypted_key = key
        .rsa()
        .expect("account key is RSA")
        .private_key_to_pem_passphrase(Cipher::aes_256_cbc(), passphrase.as_bytes())
        .expect("encrypt traditional RSA PEM");
    let _key_dir = configure_encrypted_key_file(&client, &encrypted_key, passphrase);

    // When key-pair authentication tries to load the encrypted key
    let error = client
        .connect()
        .expect_err("traditional encrypted PEM must be rejected");

    // Then the failure identifies the FIPS refusal and the supported conversion
    assert!(error.contains("traditional encrypted-PEM"), "{error}");
    assert!(error.contains("FIPS build"), "{error}");
    assert!(
        error.contains("openssl pkcs8 -topk8 -v2 aes-256-cbc"),
        "{error}"
    );
}

#[test]
fn should_download_large_result_on_a_fips_build() {
    // Given a FIPS connection and enough random data to exceed the inline result
    assert_fips_mode();
    let client = SnowflakeTestClient::connect_with_default_auth();
    let statement = client.new_statement();
    client.set_sql_query(
        &statement,
        "SELECT seq8() AS id, RANDSTR(100, RANDOM()) AS payload \
         FROM TABLE(GENERATOR(ROWCOUNT => 500000))",
    );

    // When the query produces multiple chunks, including cloud-storage downloads
    let result = client.execute_statement_query(&statement);
    let result_set = unwrap_single_rs_handle(&result);
    let chunks = client.result_set_get_chunks(&result_set).chunks;
    assert!(chunks.len() > 1, "large result must span multiple chunks");
    assert!(
        chunks
            .iter()
            .any(|chunk| matches!(&chunk.data, Some(result_chunk::Data::Remote(_)))),
        "large result must exercise the storage HTTP client"
    );
    let mut rows = ArrowResultHelper::from_result(client.result_set_get_stream(&result_set));
    let mut total_rows = 0;
    while let Some(batch) = rows.next_batch() {
        total_rows += batch.num_rows();
    }

    // Then every generated row is returned across all result chunks
    assert_eq!(total_rows, 500_000);
}

fn configured_private_key(client: &SnowflakeTestClient) -> PKey<Private> {
    let pem = get_private_key_pem_from_parameters(&client.parameters)
        .expect("read the configured account private key");
    match &client.parameters.private_key_password {
        Some(passphrase) => {
            PKey::private_key_from_pem_passphrase(pem.as_bytes(), passphrase.as_bytes())
        }
        None => PKey::private_key_from_pem(pem.as_bytes()),
    }
    .expect("parse the configured account private key")
}

fn configure_encrypted_key_file(
    client: &SnowflakeTestClient,
    encrypted_key: &[u8],
    passphrase: &str,
) -> tempfile::TempDir {
    let key_dir = tempfile::TempDir::new().expect("create encrypted-key directory");
    let key_path = key_dir.path().join("encrypted_key.p8");
    let mut key_file = tempfile::NamedTempFile::new_in(key_dir.path())
        .expect("create owner-only private-key file");
    key_file
        .write_all(encrypted_key)
        .expect("write encrypted private key");
    key_file
        .persist(&key_path)
        .expect("keep encrypted private key");
    client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
    client.set_connection_option(
        "private_key_file",
        key_path.to_str().expect("key path is UTF-8"),
    );
    client.set_connection_option("private_key_password", passphrase);
    key_dir
}

fn assert_fips_mode() {
    assert!(
        aws_lc_rs::try_fips_mode().is_ok(),
        "aws-lc is linked but not in FIPS mode"
    );
}

fn select_one(client: &SnowflakeTestClient) -> Vec<Vec<i64>> {
    let result = client.execute_query("SELECT 1");
    ArrowResultHelper::from_result(result)
        .transform_into_array::<i64>()
        .expect("SELECT 1 should return rows")
}
