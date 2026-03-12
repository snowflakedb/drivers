//! Snowflake connection management helpers

type Result<T> = std::result::Result<T, String>;
use arrow_array::StringArray;
use sf_core::protobuf::apis::RustTransport;
use sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf::generated::database_driver_v1::*;
use sf_core::rest::snowflake::STATEMENT_ASYNC_EXECUTION_OPTION;
use std::fs;

use crate::types::TestConnectionParams;

pub type DatabaseDriver = DatabaseDriverClient;

pub struct DriverRuntime {
    runtime: tokio::runtime::Runtime,
    client: DatabaseDriver,
}

impl DriverRuntime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");
        let client = DatabaseDriver::new(RustTransport::new());
        Self { runtime, client }
    }

    pub fn block_on<T>(&self, f: impl AsyncFnOnce(&DatabaseDriver) -> T) -> T {
        self.runtime.block_on(f(&self.client))
    }
}

pub fn create_database(rt: &DriverRuntime) -> Result<DatabaseHandle> {
    let db_response = rt
        .block_on(async |c| c.database_new(DatabaseNewRequest {}).await)
        .map_err(|e| format!("Database creation failed: {:?}", e))?;

    let db_handle = db_response
        .db_handle
        .ok_or_else(|| "Database creation failed: No handle returned".to_string())?;

    rt.block_on(async |c| {
        c.database_init(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .await
    })
    .map_err(|e| format!("Database initialization failed: {:?}", e))?;

    Ok(db_handle)
}

pub fn create_connection(
    rt: &DriverRuntime,
    db_handle: DatabaseHandle,
    params: &TestConnectionParams,
) -> Result<ConnectionHandle> {
    let conn_response = rt
        .block_on(async |c| c.connection_new(ConnectionNewRequest {}).await)
        .map_err(|e| format!("Connection creation failed: {:?}", e))?;

    let conn_handle = conn_response
        .conn_handle
        .ok_or_else(|| "Connection creation failed: No handle returned".to_string())?;

    set_connection_option(rt, &conn_handle, "account", &params.account)?;
    set_connection_option(rt, &conn_handle, "user", &params.user)?;

    set_connection_option(rt, &conn_handle, "authenticator", "SNOWFLAKE_JWT")?;

    let private_key_file = if let Some(ref key_file_path) = params.private_key_file {
        key_file_path.clone()
    } else if let Some(ref key_contents) = params.private_key_contents {
        write_private_key_to_file(key_contents)?
    } else {
        return Err("Neither SNOWFLAKE_TEST_PRIVATE_KEY_FILE nor SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS provided".to_string());
    };

    set_connection_option(rt, &conn_handle, "private_key_file", &private_key_file)?;
    if let Some(password) = &params.private_key_password {
        set_connection_option(rt, &conn_handle, "private_key_password", password)?;
    }

    set_connection_option(rt, &conn_handle, "database", &params.database)?;
    set_connection_option(rt, &conn_handle, "schema", &params.schema)?;
    set_connection_option(rt, &conn_handle, "warehouse", &params.warehouse)?;
    set_connection_option(rt, &conn_handle, "role", &params.role)?;
    set_connection_option(rt, &conn_handle, "host", &params.host)?;

    set_tls_options(rt, &conn_handle, params)?;

    rt.block_on(async |c| {
        c.connection_init(ConnectionInitRequest {
            conn_handle: Some(conn_handle),
            db_handle: Some(db_handle),
        })
        .await
    })
    .map_err(|e| format!("Connection initialization failed: {:?}", e))?;

    Ok(conn_handle)
}

pub fn create_statement(
    rt: &DriverRuntime,
    conn_handle: ConnectionHandle,
    sql: &str,
    async_override: Option<bool>,
) -> Result<StatementHandle> {
    let stmt_response = rt
        .block_on(async |c| {
            c.statement_new(StatementNewRequest {
                conn_handle: Some(conn_handle),
            })
            .await
        })
        .map_err(|e| format!("Statement creation failed: {:?}", e))?;

    let stmt_handle = stmt_response
        .stmt_handle
        .ok_or_else(|| "Statement creation failed: No handle returned".to_string())?;

    rt.block_on(async |c| {
        c.statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .await
    })
    .map_err(|e| format!("Statement SQL query set failed: {:?}", e))?;

    if let Some(enabled) = async_override {
        let value = if enabled { "true" } else { "false" }.to_string();
        rt.block_on(async |c| {
            c.statement_set_option_string(StatementSetOptionStringRequest {
                stmt_handle: Some(stmt_handle),
                key: STATEMENT_ASYNC_EXECUTION_OPTION.to_string(),
                value,
            })
            .await
        })
        .map_err(|e| format!("Statement option set failed: {:?}", e))?;
    }

    Ok(stmt_handle)
}

pub fn reset_statement_query(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
) -> Result<()> {
    rt.block_on(async |c| {
        c.statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .await
    })
    .map_err(|e| format!("Statement query reset failed: {:?}", e))?;
    Ok(())
}

pub fn get_server_version(rt: &DriverRuntime, conn_handle: ConnectionHandle) -> Result<String> {
    use crate::arrow::create_arrow_reader;

    let version_stmt =
        create_statement(rt, conn_handle, "SELECT CURRENT_VERSION() AS VERSION", None)?;

    let response = rt
        .block_on(async |c| {
            c.statement_execute_query(StatementExecuteQueryRequest {
                stmt_handle: Some(version_stmt),
                bindings: None,
            })
            .await
        })
        .map_err(|e| format!("Query execution failed: {:?}", e))?;

    let result = response
        .result
        .ok_or_else(|| "No result in execute response".to_string())?;

    // Arrow iteration happens outside the runtime
    let mut reader = create_arrow_reader(result)?;

    if let Some(batch_result) = reader.next() {
        let batch = batch_result.map_err(|e| format!("Failed to read batch: {:?}", e))?;

        if let Some(column) = batch.column(0).as_any().downcast_ref::<StringArray>() {
            if batch.num_rows() > 0 {
                let version = column.value(0).to_string();

                rt.block_on(async |c| {
                    c.statement_release(StatementReleaseRequest {
                        stmt_handle: Some(version_stmt),
                    })
                    .await
                })
                .ok();

                return Ok(version);
            }
        }
    }

    rt.block_on(async |c| {
        c.statement_release(StatementReleaseRequest {
            stmt_handle: Some(version_stmt),
        })
        .await
    })
    .ok();

    Err(format!("Could not extract version from result"))
}

pub fn execute_setup_queries(
    rt: &DriverRuntime,
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

        let stmt_handle = create_statement(rt, conn_handle, query, None)
            .map_err(|e| format!("Setup query statement creation failed: {:?}", e))?;

        rt.block_on(async |c| {
            c.statement_execute_query(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
            })
            .await
        })
        .map_err(|e| format!("Setup query execution failed: {:?}", e))?;

        rt.block_on(async |c| {
            c.statement_release(StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
            })
            .await
        })
        .ok();
    }

    println!("✓ Setup queries completed");
    Ok(())
}

fn set_connection_option(
    rt: &DriverRuntime,
    conn_handle: &ConnectionHandle,
    key: &str,
    value: &str,
) -> Result<()> {
    rt.block_on(async |c| {
        c.connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(*conn_handle),
            key: key.to_string(),
            value: value.to_string(),
        })
        .await
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

fn set_tls_options(
    rt: &DriverRuntime,
    conn_handle: &ConnectionHandle,
    params: &TestConnectionParams,
) -> Result<()> {
    if let Some(ref cert_path) = params.custom_root_store_path {
        set_connection_option(rt, conn_handle, "custom_root_store_path", cert_path)?;
    }
    if let Some(ref verify_certs) = params.verify_certificates {
        set_connection_option(rt, conn_handle, "verify_certificates", verify_certs)?;
    }
    if let Some(ref verify_host) = params.verify_hostname {
        set_connection_option(rt, conn_handle, "verify_hostname", verify_host)?;
    }
    if let Some(ref crl_mode) = params.crl_check_mode {
        set_connection_option(rt, conn_handle, "crl_check_mode", crl_mode)?;
    }
    Ok(())
}
