use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use napi::bindgen_prelude::{Buffer, Null, ToNapiValue};
use napi::{Result, sys};

/// Intermediate cell value between Arrow decoding and the JS boundary.
///
/// Row decoding runs off the main Node.js thread (so we don't block the
/// event loop). Real napi/JS values can only be created on that main
/// thread, so decoders produce this plain Rust enum instead; it is turned
/// into JS (`string` / `boolean` / `null` / etc) when the result is delivered
/// back to JavaScript.
pub enum SqlValue {
    Null,
    Bool(bool),
    String(String),
    Binary(Vec<u8>),
    Date(NaiveDate),
}

impl ToNapiValue for SqlValue {
    unsafe fn to_napi_value(env: sys::napi_env, sql_value: Self) -> Result<sys::napi_value> {
        match sql_value {
            SqlValue::Null => unsafe { Null::to_napi_value(env, Null) },
            SqlValue::Bool(val) => unsafe { bool::to_napi_value(env, val) },
            SqlValue::String(val) => unsafe { String::to_napi_value(env, val) },
            SqlValue::Binary(bytes) => unsafe { Buffer::to_napi_value(env, bytes.into()) },
            SqlValue::Date(date) => unsafe {
                NaiveDateTime::to_napi_value(env, date.and_time(NaiveTime::MIN))
            },
        }
    }
}
