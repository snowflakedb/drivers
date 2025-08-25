use crate::api::{OdbcError, OdbcResult, env_from_handle};
use odbc_sys as sql;
use tracing;

fn to_env_attr(attribute: i32) -> Option<sql::EnvironmentAttribute> {
    match attribute {
        200 => Some(sql::EnvironmentAttribute::OdbcVersion),
        201 => Some(sql::EnvironmentAttribute::ConnectionPooling),
        202 => Some(sql::EnvironmentAttribute::CpMatch),
        10001 => Some(sql::EnvironmentAttribute::OutputNts),
        _ => None,
    }
}

/// Set an environment attribute
pub fn set_env_attribute(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
) -> OdbcResult<()> {
    tracing::debug!("Setting environment attribute: {}", attribute);

    let env = env_from_handle(environment_handle);
    let attr = to_env_attr(attribute).ok_or(OdbcError::UnknownAttribute(attribute))?;

    match attr {
        sql::EnvironmentAttribute::OdbcVersion => {
            tracing::debug!("Setting ODBC version: {:?}", value);
            let int = value as sql::Integer;
            env.odbc_version = int;
            Ok(())
        }
        _ => {
            tracing::error!("Unhandled environment attribute: {:?}", attribute);
            Err(OdbcError::UnknownAttribute(attribute))
        }
    }
}

/// Get an environment attribute
pub fn get_env_attribute(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
) -> OdbcResult<()> {
    tracing::debug!("Getting environment attribute: {}", attribute);

    let env = env_from_handle(environment_handle);
    let attr = to_env_attr(attribute).ok_or(OdbcError::UnknownAttribute(attribute))?;

    match attr {
        sql::EnvironmentAttribute::OdbcVersion => {
            tracing::debug!("Getting ODBC version");
            let int_ptr = value as *mut sql::Integer;
            unsafe {
                std::ptr::write(int_ptr, env.odbc_version);
            }
            tracing::debug!("ODBC version: {}", env.odbc_version);
            Ok(())
        }
        _ => {
            tracing::error!("Unhandled environment attribute: {:?}", attribute);
            Err(OdbcError::UnknownAttribute(attribute))
        }
    }
}
