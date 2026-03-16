use crate::api::{
    OdbcResult, env_from_handle,
    error::{InvalidAttributeValueSnafu, ReadOnlyAttributeSnafu, UnsupportedAttributeSnafu},
};
use odbc_sys as sql;

pub(crate) const SQL_TRUE: sql::Integer = 1;

const SQL_OV_ODBC2: sql::Integer = 2;
const SQL_OV_ODBC3: sql::Integer = 3;
const SQL_OV_ODBC3_80: sql::Integer = 380;

fn to_env_attr(attribute: i32) -> Option<sql::EnvironmentAttribute> {
    match attribute {
        200 => Some(sql::EnvironmentAttribute::OdbcVersion),
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

    let env = env_from_handle(environment_handle);
    let attr = to_env_attr(attribute).ok_or(UnsupportedAttributeSnafu { attribute }.build())?;

    match attr {
        sql::EnvironmentAttribute::OdbcVersion => {
            let version = value as sql::Integer;
            match version {
                SQL_OV_ODBC2 | SQL_OV_ODBC3 | SQL_OV_ODBC3_80 => {
                    tracing::debug!("Setting ODBC version: {version}");
                    env.odbc_version = version;
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
            env.connection_pooling = pooling;
            Ok(())
        }
        sql::EnvironmentAttribute::CpMatch => {
            let connection_pool_match =
                parse_connection_pool_match(value as sql::UInteger, attribute)?;
            tracing::debug!("Setting connection pool match: {connection_pool_match:?}");
            env.connection_pool_match = connection_pool_match;
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

    let env = env_from_handle(environment_handle);
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
            tracing::debug!("Getting ODBC version: {}", env.odbc_version);
            if !value.is_null() {
                unsafe { std::ptr::write(value as *mut sql::Integer, env.odbc_version) };
            }
            write_string_length();
            Ok(())
        }
        sql::EnvironmentAttribute::ConnectionPooling => {
            tracing::debug!("Getting connection pooling: {:?}", env.connection_pooling);
            if !value.is_null() {
                unsafe {
                    std::ptr::write(
                        value as *mut sql::UInteger,
                        env.connection_pooling as sql::UInteger,
                    )
                };
            }
            write_string_length();
            Ok(())
        }
        sql::EnvironmentAttribute::CpMatch => {
            tracing::debug!(
                "Getting connection pool match: {:?}",
                env.connection_pool_match
            );
            if !value.is_null() {
                unsafe {
                    std::ptr::write(
                        value as *mut sql::UInteger,
                        env.connection_pool_match as sql::UInteger,
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
