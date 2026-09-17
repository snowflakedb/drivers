use crate::api::{
    OdbcResult, env_from_handle,
    error::{InvalidAttributeValueSnafu, ReadOnlyAttributeSnafu, UnsupportedAttributeSnafu},
};
use odbc_sys as sql;

pub(crate) const SQL_TRUE: sql::Integer = 1;

const SQL_ATTR_ODBC_VERSION: sql::Integer = 200;
pub(crate) const SQL_OV_ODBC2: sql::Integer = 2;
pub(crate) const SQL_OV_ODBC3: sql::Integer = 3;
pub(crate) const SQL_OV_ODBC3_80: sql::Integer = 380;

fn to_env_attr(attribute: i32) -> Option<sql::EnvironmentAttribute> {
    match attribute {
        SQL_ATTR_ODBC_VERSION => Some(sql::EnvironmentAttribute::OdbcVersion),
        201 => Some(sql::EnvironmentAttribute::ConnectionPooling),
        202 => Some(sql::EnvironmentAttribute::CpMatch),
        10001 => Some(sql::EnvironmentAttribute::OutputNts),
        _ => None,
    }
}

fn parse_connection_pooling(
    value: sql::UInteger,
    attribute: sql::Integer,
) -> OdbcResult<sql::AttrConnectionPooling> {
    match value {
        0 => Ok(sql::AttrConnectionPooling::Off),
        1 => Ok(sql::AttrConnectionPooling::OnePerDriver),
        2 => Ok(sql::AttrConnectionPooling::OnePerHenv),
        3 => Ok(sql::AttrConnectionPooling::DriverAware),
        _ => InvalidAttributeValueSnafu {
            attribute,
            value: value as i64,
        }
        .fail(),
    }
}

fn parse_connection_pool_match(
    value: sql::UInteger,
    attribute: sql::Integer,
) -> OdbcResult<sql::AttrCpMatch> {
    match value {
        0 => Ok(sql::AttrCpMatch::Strict),
        1 => Ok(sql::AttrCpMatch::Relaxed),
        _ => InvalidAttributeValueSnafu {
            attribute,
            value: value as i64,
        }
        .fail(),
    }
}

pub fn set_env_attribute(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    _string_length: sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("Setting environment attribute: {attribute}");

    let env = env_from_handle(environment_handle)?;
    let mut environment = env.environment.lock();
    let attr = to_env_attr(attribute).ok_or(UnsupportedAttributeSnafu { attribute }.build())?;

    match attr {
        sql::EnvironmentAttribute::OdbcVersion => {
            let version = value as sql::Integer;
            match version {
                SQL_OV_ODBC2 | SQL_OV_ODBC3 | SQL_OV_ODBC3_80 => {
                    tracing::debug!("Setting ODBC version: {version}");
                    environment.odbc_version = version;
                    Ok(())
                }
                _ => {
                    tracing::warn!("Invalid ODBC version value: {version}");
                    InvalidAttributeValueSnafu {
                        attribute,
                        value: version as i64,
                    }
                    .fail()
                }
            }
        }
        sql::EnvironmentAttribute::ConnectionPooling => {
            let pooling = parse_connection_pooling(value as sql::UInteger, attribute)?;
            tracing::debug!("Setting connection pooling: {pooling:?}");
            environment.connection_pooling = pooling;
            Ok(())
        }
        sql::EnvironmentAttribute::CpMatch => {
            let connection_pool_match =
                parse_connection_pool_match(value as sql::UInteger, attribute)?;
            tracing::debug!("Setting connection pool match: {connection_pool_match:?}");
            environment.connection_pool_match = connection_pool_match;
            Ok(())
        }
        sql::EnvironmentAttribute::OutputNts => {
            tracing::warn!("SQL_ATTR_OUTPUT_NTS is read-only");
            ReadOnlyAttributeSnafu { attribute }.fail()
        }
    }
}

pub fn get_env_attribute(
    environment_handle: sql::Handle,
    attribute: sql::Integer,
    value: sql::Pointer,
    _buffer_length: sql::Integer,
    string_length_ptr: *mut sql::Integer,
) -> OdbcResult<()> {
    tracing::debug!("Getting environment attribute: {attribute}");

    let env = env_from_handle(environment_handle)?;
    let environment = env.environment.lock();
    let attr = to_env_attr(attribute).ok_or(UnsupportedAttributeSnafu { attribute }.build())?;

    let write_string_length = || {
        if !string_length_ptr.is_null() {
            unsafe {
                std::ptr::write(
                    string_length_ptr,
                    std::mem::size_of::<sql::Integer>() as sql::Integer,
                );
            }
        }
    };

    match attr {
        sql::EnvironmentAttribute::OdbcVersion => {
            tracing::debug!("Getting ODBC version: {}", environment.odbc_version);
            if !value.is_null() {
                unsafe { std::ptr::write(value as *mut sql::Integer, environment.odbc_version) };
            }
            write_string_length();
            Ok(())
        }
        sql::EnvironmentAttribute::ConnectionPooling => {
            tracing::debug!(
                "Getting connection pooling: {:?}",
                environment.connection_pooling
            );
            if !value.is_null() {
                unsafe {
                    std::ptr::write(
                        value as *mut sql::UInteger,
                        environment.connection_pooling as sql::UInteger,
                    )
                };
            }
            write_string_length();
            Ok(())
        }
        sql::EnvironmentAttribute::CpMatch => {
            tracing::debug!(
                "Getting connection pool match: {:?}",
                environment.connection_pool_match
            );
            if !value.is_null() {
                unsafe {
                    std::ptr::write(
                        value as *mut sql::UInteger,
                        environment.connection_pool_match as sql::UInteger,
                    )
                };
            }
            write_string_length();
            Ok(())
        }
        sql::EnvironmentAttribute::OutputNts => {
            tracing::debug!("Getting output NTS: {SQL_TRUE}");
            if !value.is_null() {
                unsafe { std::ptr::write(value as *mut sql::Integer, SQL_TRUE) };
            }
            write_string_length();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::handle_allocation::{alloc_environment, free_environment};

    fn get_odbc_version(env_handle: sql::Handle) -> sql::Integer {
        let mut value: sql::Integer = 0;
        get_env_attribute(
            env_handle,
            SQL_ATTR_ODBC_VERSION,
            &mut value as *mut sql::Integer as sql::Pointer,
            std::mem::size_of::<sql::Integer>() as sql::Integer,
            std::ptr::null_mut(),
        )
        .expect("get_env_attribute(OdbcVersion)");
        value
    }

    #[test]
    fn odbc_version_defaults_to_odbc3() {
        let _guard = crate::api::HANDLE_ALLOC_TEST_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let env_handle = alloc_environment().expect("alloc_environment");

        assert_eq!(get_odbc_version(env_handle), SQL_OV_ODBC3);

        free_environment(env_handle).expect("free_environment");
    }

    #[test]
    fn odbc_version_round_trips_to_odbc2_and_back() {
        // Mirrors the old ODBC driver's "Test compatibility" case: an app
        // declaring itself ODBC 2.x via SQL_ATTR_ODBC_VERSION must have that
        // declaration accepted and faithfully echoed back by SQLGetEnvAttr.
        let _guard = crate::api::HANDLE_ALLOC_TEST_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let env_handle = alloc_environment().expect("alloc_environment");

        set_env_attribute(
            env_handle,
            SQL_ATTR_ODBC_VERSION,
            SQL_OV_ODBC2 as sql::Pointer,
            0,
        )
        .expect("set_env_attribute(OdbcVersion, ODBC2)");
        assert_eq!(get_odbc_version(env_handle), SQL_OV_ODBC2);

        set_env_attribute(
            env_handle,
            SQL_ATTR_ODBC_VERSION,
            SQL_OV_ODBC3 as sql::Pointer,
            0,
        )
        .expect("set_env_attribute(OdbcVersion, ODBC3)");
        assert_eq!(get_odbc_version(env_handle), SQL_OV_ODBC3);

        free_environment(env_handle).expect("free_environment");
    }

    #[test]
    fn odbc_version_rejects_unrecognized_value() {
        let _guard = crate::api::HANDLE_ALLOC_TEST_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let env_handle = alloc_environment().expect("alloc_environment");

        let err = set_env_attribute(env_handle, SQL_ATTR_ODBC_VERSION, 99 as sql::Pointer, 0)
            .expect_err("unrecognized ODBC version must be rejected");
        assert!(matches!(
            err,
            crate::api::error::OdbcError::InvalidAttributeValue { .. }
        ));

        free_environment(env_handle).expect("free_environment");
    }
}
