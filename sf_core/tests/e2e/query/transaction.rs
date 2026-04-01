use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[test]
fn should_commit_transaction() {
    // Given a connected Snowflake client with autocommit disabled
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_commit_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query("ALTER SESSION SET AUTOCOMMIT=FALSE");

    // When a row is inserted and committed
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (1)"));
    client.commit();

    // Then the row is visible after commit
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 1);
}

#[test]
fn should_rollback_transaction() {
    // Given a connected Snowflake client with autocommit disabled and a committed row
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_rollback_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query("ALTER SESSION SET AUTOCOMMIT=FALSE");

    client.execute_query(&format!("INSERT INTO {table_name} VALUES (1)"));
    client.commit();

    // When a second row is inserted and rolled back
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (2)"));
    client.rollback();

    // Then only the first committed row remains
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 1);
}

#[test]
fn should_commit_with_no_pending_changes() {
    // Given a connected Snowflake client with autocommit disabled and an empty table
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_commit_noop_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query("ALTER SESSION SET AUTOCOMMIT=FALSE");

    // When commit is called with no pending changes
    client.commit();

    // Then the table remains empty and no error occurs
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 0);
}

#[test]
fn should_rollback_with_no_pending_changes() {
    // Given a connected Snowflake client with autocommit disabled and an empty table
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_rollback_noop_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query("ALTER SESSION SET AUTOCOMMIT=FALSE");

    // When rollback is called with no pending changes
    client.rollback();

    // Then the table remains empty and no error occurs
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 0);
}

#[test]
fn should_commit_multiple_inserts_in_single_transaction() {
    // Given a connected Snowflake client with autocommit disabled
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_multi_insert_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query("ALTER SESSION SET AUTOCOMMIT=FALSE");

    // When multiple rows are inserted and committed in a single transaction
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (1)"));
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (2)"));
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (3)"));
    client.commit();

    // Then all rows are visible after commit
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 3);
}

#[test]
fn should_commit_and_rollback_with_autocommit_enabled() {
    // Given a connected Snowflake client with autocommit enabled
    let client = SnowflakeTestClient::connect_with_default_auth();
    let table_name = format!("test_autocommit_{}", std::process::id());
    client.execute_query(&format!(
        "CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT)"
    ));
    client.execute_query(&format!("INSERT INTO {table_name} VALUES (1)"));

    // When commit and rollback are called with autocommit enabled
    client.commit();
    client.rollback();

    // Then the auto-committed row is still visible
    let result = client.execute_query(&format!("SELECT COUNT(*) FROM {table_name}"));
    let mut helper = ArrowResultHelper::from_result(result);
    let rows = helper.transform_into_array::<i64>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], 1);
}
