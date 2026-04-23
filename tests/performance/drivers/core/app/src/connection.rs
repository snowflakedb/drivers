//! Snowflake connection management helpers

type Result<T> = std::result::Result<T, String>;
use arrow_array::StringArray;
use sf_core::config::param_names;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient as BlockingDatabaseDriver, DatabaseDriverClientBlockingExt,
    database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;
use std::fs;

use crate::types::TestConnectionParams;

pub type DatabaseDriver = BlockingDatabaseDriver;

pub struct DriverRuntime {
    client: DatabaseDriver,
}

impl DriverRuntime {
    pub fn new() -> Self {
        let client = database_driver_client();
        Self { client }
    }

    pub fn client(&self) -> &DatabaseDriver {
        &self.client
    }
}

pub fn create_database(rt: &DriverRuntime) -> Result<DatabaseHandle> {
    let db_response = rt
        .client
        .database_new_blocking(DatabaseNewRequest {})
        .map_err(|e| format!("Database creation failed: {:?}", e))?;

    let db_handle = db_response
        .db_handle
        .ok_or_else(|| "Database creation failed: No handle returned".to_string())?;

    rt.client
        .database_init_blocking(DatabaseInitRequest {
            db_handle: Some(db_handle),
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
        .client
        .connection_new_blocking(ConnectionNewRequest {})
        .map_err(|e| format!("Connection creation failed: {:?}", e))?;

    let conn_handle = conn_response
        .conn_handle
        .ok_or_else(|| "Connection creation failed: No handle returned".to_string())?;

    // Set connection parameters
    set_connection_option(rt, &conn_handle, "account", &params.account)?;
    set_connection_option(rt, &conn_handle, "user", &params.user)?;

    // Use JWT key-pair authentication
    set_connection_option(rt, &conn_handle, "authenticator", "SNOWFLAKE_JWT")?;

    // First check if a private key file path is provided, otherwise create from contents
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

    // Initialize connection (performs login)
    rt.client
        .connection_init_blocking(ConnectionInitRequest {
            conn_handle: Some(conn_handle),
            db_handle: Some(db_handle),
            ..Default::default()
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
        .client
        .statement_new_blocking(StatementNewRequest {
            conn_handle: Some(conn_handle),
        })
        .map_err(|e| format!("Statement creation failed: {:?}", e))?;

    let stmt_handle = stmt_response
        .stmt_handle
        .ok_or_else(|| "Statement creation failed: No handle returned".to_string())?;

    rt.client
        .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .map_err(|e| format!("Statement SQL query set failed: {:?}", e))?;

    if let Some(enabled) = async_override {
        let options = [(
            param_names::ASYNC_EXECUTION.as_str().to_string(),
            ConfigSetting::from(enabled),
        )]
        .into_iter()
        .collect();
        rt.client
            .statement_set_options_blocking(StatementSetOptionsRequest {
                stmt_handle: Some(stmt_handle),
                options,
            })
            .map_err(|e| format!("Statement option set failed: {:?}", e))?;
    }

    Ok(stmt_handle)
}

pub fn reset_statement_query(rt: &DriverRuntime, stmt_handle: StatementHandle, sql: &str) -> Result<()> {
    rt.client
        .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .map_err(|e| format!("Statement query reset failed: {:?}", e))?;
    Ok(())
}

pub fn get_server_version(rt: &DriverRuntime, conn_handle: ConnectionHandle) -> Result<String> {
    use crate::arrow::create_arrow_reader;

    let version_stmt = create_statement(rt, conn_handle, "SELECT CURRENT_VERSION() AS VERSION", None)?;
    let response = rt
        .client
        .statement_execute_query_blocking(StatementExecuteQueryRequest {
            stmt_handle: Some(version_stmt),
            bindings: None,
        })
        .map_err(|e| format!("Query execution failed: {:?}", e))?;

    let query_id = match response.result {
        Some(execute_query_response::Result::Single(descriptor)) => descriptor.query_id,
        _ => return Err("Unexpected result type from server version query".to_string()),
    };
    let result_set = rt
        .client
        .statement_get_result_set_blocking(StatementGetResultSetRequest {
            stmt_handle: Some(version_stmt),
            query_id,
        })
        .map_err(|e| format!("Failed to get result set: {:?}", e))?;
    let mut reader = create_arrow_reader(result_set)?;

    if let Some(batch_result) = reader.next() {
        let batch = batch_result.map_err(|e| format!("Failed to read batch: {:?}", e))?;

        if let Some(column) = batch.column(0).as_any().downcast_ref::<StringArray>() {
            if batch.num_rows() > 0 {
                let version = column.value(0).to_string();

                rt.client
                    .statement_release_blocking(StatementReleaseRequest {
                        stmt_handle: Some(version_stmt),
                    })
                    .ok();

                return Ok(version);
            }
        }
    }

    rt.client
        .statement_release_blocking(StatementReleaseRequest {
            stmt_handle: Some(version_stmt),
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

        rt.client
            .statement_execute_query_blocking(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
            })
            .map_err(|e| format!("Setup query execution failed: {:?}", e))?;

        rt.client
            .statement_release_blocking(StatementReleaseRequest {
                stmt_handle: Some(stmt_handle),
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
    let options = [(key.to_string(), ConfigSetting::from(value))]
        .into_iter()
        .collect();
    rt.client
        .connection_set_options_blocking(ConnectionSetOptionsRequest {
            conn_handle: Some(*conn_handle),
            options,
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
