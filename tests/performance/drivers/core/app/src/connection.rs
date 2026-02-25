//! Snowflake connection management helpers

type Result<T> = std::result::Result<T, String>;
use arrow_array::StringArray;
use sf_core::protobuf_apis::RustTransport;
use sf_core::protobuf_gen::database_driver_v1::*;
use sf_core::rest::snowflake::STATEMENT_ASYNC_EXECUTION_OPTION;
use std::fs;

use crate::types::TestConnectionParams;

pub type DatabaseDriver = DatabaseDriverClient<RustTransport>;

pub fn create_database() -> Result<DatabaseHandle> {
    let db_response = DatabaseDriver::database_new(DatabaseNewRequest {})
        .map_err(|e| format!("Database creation failed: {:?}", e))?;

    let db_handle = db_response
        .db_handle
        .ok_or_else(|| "Database creation failed: No handle returned".to_string())?;

    DatabaseDriver::database_init(DatabaseInitRequest {
        db_handle: Some(db_handle),
    })
    .map_err(|e| format!("Database initialization failed: {:?}", e))?;

    Ok(db_handle)
}

pub fn create_connection(
    db_handle: DatabaseHandle,
    params: &TestConnectionParams,
) -> Result<ConnectionHandle> {
    let conn_response = DatabaseDriver::connection_new(ConnectionNewRequest {})
        .map_err(|e| format!("Connection creation failed: {:?}", e))?;

    let conn_handle = conn_response
        .conn_handle
        .ok_or_else(|| "Connection creation failed: No handle returned".to_string())?;

    // Set connection parameters
    set_connection_option(&conn_handle, "account", &params.account)?;
    set_connection_option(&conn_handle, "user", &params.user)?;

    // Use JWT key-pair authentication
    set_connection_option(&conn_handle, "authenticator", "SNOWFLAKE_JWT")?;
    let private_key_file = write_private_key_to_file(&params.private_key_contents)?;
    set_connection_option(&conn_handle, "private_key_file", &private_key_file)?;
    if let Some(password) = &params.private_key_password {
        set_connection_option(&conn_handle, "private_key_password", password)?;
    }

    set_connection_option(&conn_handle, "database", &params.database)?;
    set_connection_option(&conn_handle, "schema", &params.schema)?;
    set_connection_option(&conn_handle, "warehouse", &params.warehouse)?;
    set_connection_option(&conn_handle, "role", &params.role)?;
    set_connection_option(&conn_handle, "host", &params.host)?;

    set_tls_options(&conn_handle, params)?;

    // Initialize connection (performs login)
    DatabaseDriver::connection_init(ConnectionInitRequest {
        conn_handle: Some(conn_handle),
        db_handle: Some(db_handle),
    })
    .map_err(|e| format!("Connection initialization failed: {:?}", e))?;

    Ok(conn_handle)
}

pub fn create_statement(
    conn_handle: ConnectionHandle,
    sql: &str,
    async_override: Option<bool>,
) -> Result<StatementHandle> {
    let stmt_response = DatabaseDriver::statement_new(StatementNewRequest {
        conn_handle: Some(conn_handle),
    })
    .map_err(|e| format!("Statement creation failed: {:?}", e))?;

    let stmt_handle = stmt_response
        .stmt_handle
        .ok_or_else(|| "Statement creation failed: No handle returned".to_string())?;

    DatabaseDriver::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle),
        query: sql.to_string(),
    })
    .map_err(|e| format!("Statement SQL query set failed: {:?}", e))?;

    if let Some(enabled) = async_override {
        let value = if enabled { "true" } else { "false" }.to_string();
        DatabaseDriver::statement_set_option_string(StatementSetOptionStringRequest {
            stmt_handle: Some(stmt_handle),
            key: STATEMENT_ASYNC_EXECUTION_OPTION.to_string(),
            value,
        })
        .map_err(|e| format!("Statement option set failed: {:?}", e))?;
    }

    Ok(stmt_handle)
}

pub fn reset_statement_query(stmt_handle: StatementHandle, sql: &str) -> Result<()> {
    DatabaseDriver::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle),
        query: sql.to_string(),
    })
    .map_err(|e| format!("Statement query reset failed: {:?}", e))?;
    Ok(())
}

pub fn get_server_version(conn_handle: ConnectionHandle) -> Result<String> {
    use crate::arrow::create_arrow_reader;

    let version_stmt = create_statement(conn_handle, "SELECT CURRENT_VERSION() AS VERSION", None)?;
    let response = DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(version_stmt),
        bindings: None,
    })
    .map_err(|e| format!("Query execution failed: {:?}", e))?;

    let result = response
        .result
        .ok_or_else(|| "No result in execute response".to_string())?;
    let mut reader = create_arrow_reader(result)?;

    if let Some(batch_result) = reader.next() {
        let batch = batch_result.map_err(|e| format!("Failed to read batch: {:?}", e))?;

        if let Some(column) = batch.column(0).as_any().downcast_ref::<StringArray>() {
            if batch.num_rows() > 0 {
                let version = column.value(0).to_string();

                DatabaseDriver::statement_release(StatementReleaseRequest {
                    stmt_handle: Some(version_stmt),
                })
                .ok();

                return Ok(version);
            }
        }
    }

    DatabaseDriver::statement_release(StatementReleaseRequest {
        stmt_handle: Some(version_stmt),
    })
    .ok();

    Err(format!("Could not extract version from result"))
}

pub fn execute_setup_queries(
    conn_handle: ConnectionHandle,
    setup_queries: &[String],
) -> Result<()> {
    if setup_queries.is_empty() {
        return Ok(());
    }

    println!(
        "\n=== Executing Setup Queries ({} queries) ===",
        setup_queries.len()
    );

    for (i, query) in setup_queries.iter().enumerate() {
        println!("  Setup query {}: {}", i + 1, query);

        let stmt_handle = create_statement(conn_handle, query, None)
            .map_err(|e| format!("Setup query statement creation failed: {:?}", e))?;

        DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
            bindings: None,
        })
        .map_err(|e| format!("Setup query execution failed: {:?}", e))?;

        DatabaseDriver::statement_release(StatementReleaseRequest {
            stmt_handle: Some(stmt_handle),
        })
        .ok();
    }

    println!("✓ Setup queries completed");
    Ok(())
}

fn set_connection_option(conn_handle: &ConnectionHandle, key: &str, value: &str) -> Result<()> {
    DatabaseDriver::connection_set_option_string(ConnectionSetOptionStringRequest {
        conn_handle: Some(*conn_handle),
        key: key.to_string(),
        value: value.to_string(),
    })
    .map_err(|e| format!("Connection option set failed ({}): {:?}", key, e))?;
    Ok(())
}

fn write_private_key_to_file(private_key_contents: &[String]) -> Result<String> {
    let temp_dir = std::env::temp_dir();
    let key_file_path = temp_dir.join("perf_test_private_key.p8");
    let private_key = private_key_contents.join("\n") + "\n";

    fs::write(&key_file_path, private_key)
        .map_err(|e| format!("Private key file write failed: {:?}", e))?;

    Ok(key_file_path.display().to_string())
}

fn set_tls_options(conn_handle: &ConnectionHandle, params: &TestConnectionParams) -> Result<()> {
    if let Some(ref cert_path) = params.custom_root_store_path {
        set_connection_option(conn_handle, "custom_root_store_path", cert_path)?;
    }
    if let Some(ref verify_certs) = params.verify_certificates {
        set_connection_option(conn_handle, "verify_certificates", verify_certs)?;
    }
    if let Some(ref verify_host) = params.verify_hostname {
        set_connection_option(conn_handle, "verify_hostname", verify_host)?;
    }
    if let Some(ref crl_mode) = params.crl_check_mode {
        set_connection_option(conn_handle, "crl_check_mode", crl_mode)?;
    }
    Ok(())
}
