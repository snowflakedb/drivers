use super::super::common::test_utils::*;
use sf_core::thrift_apis::DatabaseDriverV1;
use sf_core::thrift_apis::client::create_client;

#[test]
fn connection_init_with_tls_options_succeeds() {
    // Given TLS certificate and hostname verification are enabled
    let helper = SnowflakeTestClient::with_default_params();
    let mut client = create_client::<DatabaseDriverV1>();
    let db = helper.db_handle.clone();
    let conn = helper.conn_handle.clone();

    client
        .connection_set_option_string(conn.clone(), "verify_hostname".into(), "true".into())
        .unwrap();
    client
        .connection_set_option_string(conn.clone(), "verify_certificates".into(), "true".into())
        .unwrap();

    // And connection parameters (account, user, password, host) are set
    client
        .connection_set_option_string(
            conn.clone(),
            "password".to_string(),
            helper.parameters.password.clone().unwrap(),
        )
        .unwrap();

    // When I initialize the connection
    client.connection_init(conn.clone(), db.clone()).unwrap();

    // Then the login succeeds
    client.connection_release(conn).unwrap();
    client.database_release(db).unwrap();
}
