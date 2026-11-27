use crate::common::config::{get_parameters, setup_logging};
use crate::common::private_key_helper::get_private_key_pem_from_parameters;
use arrow::array::{Array, StringArray};
use sf_core::apis::database_driver_v1::{
    DatabaseDriverV1, ExecuteQueryResult, Handle, ResultSetInfo, Setting,
};
use std::collections::HashMap;
use std::time::Duration;

async fn execute(driver: &DatabaseDriverV1, conn: Handle, sql: &str) -> ResultSetInfo {
    let stmt = driver.statement_new(conn).unwrap();
    driver
        .statement_set_sql_query(stmt, sql.into())
        .await
        .unwrap();
    let result = driver
        .statement_execute_query(None, stmt, None, None, None, None)
        .await
        .unwrap();
    driver.statement_release(stmt).unwrap();
    match result {
        ExecuteQueryResult::Single { info, .. } => info,
        ExecuteQueryResult::Multi { .. } => panic!("expected one result set"),
    }
}

async fn optional_string(
    driver: &DatabaseDriverV1,
    info: ResultSetInfo,
    column: &str,
) -> Option<String> {
    let reader = driver.result_set_get_stream(info.handle).await.unwrap();
    let column = column.to_string();
    let value = tokio::task::spawn_blocking(move || {
        let batches = reader.collect::<Result<Vec<_>, _>>().unwrap();
        let row_count = batches.iter().map(|batch| batch.num_rows()).sum::<usize>();
        assert!(row_count <= 1, "expected at most one row, got {row_count}");
        let batch = batches.iter().find(|batch| batch.num_rows() == 1)?;
        let values = batch
            .column_by_name(&column)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(!values.is_null(0), "{column} must not be NULL");
        Some(values.value(0).to_string())
    })
    .await
    .unwrap();
    driver.result_set_release(info.handle).unwrap();
    value
}

async fn single_string(driver: &DatabaseDriverV1, info: ResultSetInfo, column: &str) -> String {
    optional_string(driver, info, column)
        .await
        .expect("expected exactly one row")
}

async fn query_history_tag(driver: &DatabaseDriverV1, conn: Handle, query_id: &str) -> String {
    let sql = format!(
        "SELECT QUERY_TAG FROM TABLE(INFORMATION_SCHEMA.QUERY_HISTORY_BY_SESSION()) \
         WHERE QUERY_ID = '{query_id}'"
    );
    // History is eventually consistent. Retry only a missing row, never a
    // NULL or incorrect tag, and bound the whole lookup including requests.
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let history = execute(driver, conn, &sql).await;
            if let Some(tag) = optional_string(driver, history, "QUERY_TAG").await {
                return tag;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for query {query_id} in query history"))
}

#[tokio::test(flavor = "multi_thread")]
async fn should_apply_per_upload_query_tag_to_streamed_put_without_changing_session_tag() {
    // The tagged upload API is Rust-only, so use the core driver directly.
    // Connection options mirror SnowflakeTestClient::set_options_from_parameters
    // so this test honors the same PARAMETER_PATH settings as the other live
    // tests, including host, server_url, port, and protocol individually.
    setup_logging();
    let parameters = get_parameters();
    let mut options: HashMap<String, Setting> = HashMap::new();
    for (name, value) in [
        ("account", parameters.account_name.clone()),
        ("user", parameters.user.clone()),
        ("database", parameters.database.clone()),
        ("schema", parameters.schema.clone()),
        ("warehouse", parameters.warehouse()),
        ("host", parameters.host.clone()),
        ("role", parameters.role.clone()),
        ("server_url", parameters.server_url.clone()),
        ("protocol", parameters.protocol.clone()),
    ] {
        if let Some(value) = value {
            options.insert(name.into(), Setting::String(value));
        }
    }
    if let Some(port) = parameters.port {
        options.insert("port".into(), Setting::Int(port));
    }
    if parameters.private_key_file.is_some() || parameters.private_key_contents.is_some() {
        options.insert(
            "authenticator".into(),
            Setting::String("SNOWFLAKE_JWT".into()),
        );
        options.insert(
            "private_key".into(),
            Setting::String(get_private_key_pem_from_parameters(&parameters).unwrap()),
        );
        if let Some(password) = parameters.private_key_password {
            options.insert("private_key_password".into(), Setting::String(password));
        }
    } else {
        options.insert(
            "password".into(),
            Setting::String(parameters.password.expect("live test password or JWT key")),
        );
    }
    let driver = DatabaseDriverV1::new();
    let db = driver.database_new();
    driver.database_init(db).unwrap();
    let conn = driver.connection_new();
    driver
        .connection_set_options(conn, options, false, None)
        .await
        .unwrap();
    driver.connection_init(None, conn, db).await.unwrap();
    // Given Snowflake client is logged in with session QUERY_TAG "session_tag" and a temporary stage
    for sql in [
        "CREATE TEMPORARY STAGE stream_query_tag_stage",
        "ALTER SESSION SET QUERY_TAG = 'session_tag'",
    ] {
        let result = execute(&driver, conn, sql).await;
        driver.result_set_release(result.handle).unwrap();
    }

    for (tag, expected) in [
        (None, "session_tag"),
        (Some("override"), "override"),
        (Some(""), ""),
    ] {
        // When a streamed PUT is uploaded with the per-upload query tag
        let upload = driver
            .connection_upload_stream_begin_with_query_tag(
                conn,
                "PUT file:///stream_tag.csv @stream_query_tag_stage AUTO_COMPRESS=FALSE OVERWRITE=TRUE"
                    .into(),
                tag.map(str::to_string),
            )
            .await
            .unwrap();
        driver
            .connection_upload_stream_chunk(upload, b"id,value\n1,tagged\n".to_vec())
            .await
            .unwrap();
        let put = Box::pin(driver.connection_upload_stream_finish(None, upload))
            .await
            .unwrap();
        let put_id = put.descriptor.query_id.clone();
        // Then the PUT is uploaded
        assert_eq!(single_string(&driver, put, "status").await, "UPLOADED");

        // And the next untagged query still reports the session tag
        // (it must be the first query after the PUT, without any request override)
        let select = execute(&driver, conn, "SELECT CURRENT_QUERY_TAG() AS TAG").await;
        let select_id = select.descriptor.query_id.clone();
        assert_eq!(single_string(&driver, select, "TAG").await, "session_tag");
        // And query history records the override for the PUT and the session tag for the SELECT
        for (kind, query_id, expected_tag) in [
            ("PUT", put_id, expected),
            ("subsequent SELECT", select_id, "session_tag"),
        ] {
            let actual = query_history_tag(&driver, conn, &query_id).await;
            eprintln!("upload tag={tag:?}: {kind} history tag={actual:?}, query_id={query_id}");
            assert_eq!(
                actual, expected_tag,
                "{kind} history tag for override {tag:?}"
            );
        }
    }
    driver.connection_close(None, conn).await.unwrap();
    driver.connection_release(conn).unwrap();
    driver.database_release(db).unwrap();
}
