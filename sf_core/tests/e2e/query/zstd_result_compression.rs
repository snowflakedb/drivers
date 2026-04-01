use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

/// Verifies that the driver can fetch query result chunks compressed with zstd.
///
/// Reqwest's `zstd` feature transparently decompresses HTTP responses with
/// `Content-Encoding: zstd`, so no application-level decompression code is needed.
#[test]
fn should_fetch_large_result_set_with_zstd_compression() {
    let client = SnowflakeTestClient::with_default_params();

    client.set_connection_option("client_app_id", "JDBC");

    let password = client.parameters.password.clone().unwrap();
    client.set_connection_option("password", &password);

    client.connect().expect("Connection failed");

    // Enable zstd result compression for this session
    client.execute_query("ALTER SESSION SET ENABLE_PARAMETRIZE_RESULT_COMPRESSION_TYPE = true");
    client.execute_query("ALTER SESSION SET JDBC_QUERY_RESULT_COMPRESSION_TYPE = 'ZSTD'");

    let result = client.execute_query(
        "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id",
    );

    let mut arrow_helper = ArrowResultHelper::from_result(result);
    let rows = arrow_helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1000000);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i as i64);
    }
}
