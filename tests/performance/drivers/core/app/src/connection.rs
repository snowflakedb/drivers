//! Snowflake connection management helpers

type Result<T> = std::result::Result<T, String>;
use arrow_array::StringArray;
use sf_core::protobuf_apis::RustTransport;
use sf_core::protobuf_gen::database_driver_v1::*;
use std::fs;

use crate::types::TestConnectionParams;

pub type DatabaseDriver = DatabaseDriverClient<RustTransport>;

pub fn create_database() -> Result<DatabaseHandle> {
    let db_response = DatabaseDriver::database_new(DatabaseNewRequest {})
        .map_err(|e| format!("Failed to create database: {:?}", e))?;

    let db_handle = db_response
        .db_handle
        .ok_or_else(|| "No database handle returned".to_string())?;

    DatabaseDriver::database_init(DatabaseInitRequest {
        db_handle: Some(db_handle),
    })
    .map_err(|e| format!("Failed to initialize database: {:?}", e))?;

    Ok(db_handle)
}

pub fn create_connection(
    db_handle: DatabaseHandle,
    params: &TestConnectionParams,
) -> Result<ConnectionHandle> {
    let conn_response = DatabaseDriver::connection_new(ConnectionNewRequest {})
        .map_err(|e| format!("Failed to create connection: {:?}", e))?;

    let conn_handle = conn_response
        .conn_handle
        .ok_or_else(|| "No connection handle returned".to_string())?;

    // Set connection parameters
    set_connection_option(&conn_handle, "account", &params.account)?;
    set_connection_option(&conn_handle, "user", &params.user)?;

    // Use JWT key-pair authentication
    set_connection_option(&conn_handle, "authenticator", "SNOWFLAKE_JWT")?;
    let private_key_file = write_private_key_to_file(&params.private_key_contents)?;
    set_connection_option(&conn_handle, "private_key_file", &private_key_file)?;

    set_connection_option(&conn_handle, "database", &params.database)?;
    set_connection_option(&conn_handle, "schema", &params.schema)?;
    set_connection_option(&conn_handle, "warehouse", &params.warehouse)?;
    set_connection_option(&conn_handle, "role", &params.role)?;
    set_connection_option(&conn_handle, "host", &params.host)?;

    // Initialize connection (performs login)
    DatabaseDriver::connection_init(ConnectionInitRequest {
        conn_handle: Some(conn_handle),
        db_handle: Some(db_handle),
    })
    .map_err(|e| format!("Connection failed: {:?}", e))?;

    Ok(conn_handle)
}

pub fn create_statement(conn_handle: ConnectionHandle, sql: &str) -> Result<StatementHandle> {
    let stmt_response = DatabaseDriver::statement_new(StatementNewRequest {
        conn_handle: Some(conn_handle),
    })
    .map_err(|e| format!("Failed to create statement: {:?}", e))?;

    let stmt_handle = stmt_response
        .stmt_handle
        .ok_or_else(|| "No statement handle returned".to_string())?;

    DatabaseDriver::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle),
        query: sql.to_string(),
    })
    .map_err(|e| format!("Failed to set SQL query: {:?}", e))?;

    Ok(stmt_handle)
}

pub fn reset_statement_query(stmt_handle: StatementHandle, sql: &str) -> Result<()> {
    DatabaseDriver::statement_set_sql_query(StatementSetSqlQueryRequest {
        stmt_handle: Some(stmt_handle),
        query: sql.to_string(),
    })
    .map_err(|e| format!("Failed to reset SQL query: {:?}", e))?;
    Ok(())
}

pub fn get_server_version(conn_handle: ConnectionHandle) -> Result<String> {
    use crate::arrow::create_arrow_reader;

    let version_stmt = create_statement(conn_handle, "SELECT CURRENT_VERSION() AS VERSION")?;
    let response = DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(version_stmt),
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

        let stmt_handle = create_statement(conn_handle, query)
            .map_err(|e| format!("Failed to create setup statement: {:?}", e))?;

        DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
        })
        .map_err(|e| format!("Setup query failed: {:?}", e))?;

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
    .map_err(|e| format!("Failed to set option: {:?}", e))?;
    Ok(())
}

fn write_private_key_to_file(private_key_contents: &[String]) -> Result<String> {
    let temp_dir = std::env::temp_dir();
    let key_file_path = temp_dir.join("perf_test_private_key.p8");
    let private_key = private_key_contents.join("\n") + "\n";

    fs::write(&key_file_path, private_key)
        .map_err(|e| format!("Failed to write private key file: {:?}", e))?;

    Ok(key_file_path.display().to_string())
}
