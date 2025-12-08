use crate::common::test_utils::*;

#[test]
#[ignore] // Requires Snowflake credentials
fn test_large_result_set_1m_rows() {
    let client = SnowflakeTestClient::connect_with_default_auth();
    let _result =
        client.execute_query("SELECT SEQ8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000))");
    // Test passes if query executes without error
}

#[test]
#[ignore] // Requires Snowflake credentials
fn test_wide_table_many_columns() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    // Generate a query with 100 columns
    let columns: Vec<String> = (0..100).map(|i| format!("{} as col_{}", i, i)).collect();
    let query = format!("SELECT {}", columns.join(", "));

    let _result = client.execute_query(&query);
    // Test passes if query executes without error
}

#[test]
#[ignore] // Requires Snowflake credentials
fn test_many_small_queries() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    for i in 0..100 {
        let _result = client.execute_query(&format!("SELECT {} as query_num", i));
    }
    // Test passes if all queries execute without error
}

#[test]
#[ignore] // Requires Snowflake credentials
fn test_decimal_precision_stress() {
    let client = SnowflakeTestClient::connect_with_default_auth();

    let _result = client.execute_query(
        "SELECT \
         123.45::DECIMAL(10,2) as dec1, \
         999999999999999999.99::DECIMAL(20,2) as dec2, \
         0.123456789012345678::DECIMAL(20,18) as dec3",
    );
    // Test passes if query executes without error
}
