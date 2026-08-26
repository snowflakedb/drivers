use chrono::NaiveDateTime;
use napi::bindgen_prelude::{Buffer, Null, ToNapiValue};
use napi::{Result, sys};
use std::borrow::Cow;

/// Rust stand-in for a JS cell so Arrow conversion can be asserted without napi env
#[derive(Debug, PartialEq)]
pub(crate) enum JsCell<'a> {
    Null,
    Bool(bool),
    Str(Cow<'a, str>),
    Number(f64),
    Buffer(&'a [u8]),
    Date(NaiveDateTime),
}

impl<'a> ToNapiValue for JsCell<'a> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        match val {
            JsCell::Null => unsafe { Null::to_napi_value(env, Null) },
            JsCell::Bool(val) => unsafe { bool::to_napi_value(env, val) },
            JsCell::Str(val) => unsafe { ToNapiValue::to_napi_value(env, val.as_ref()) },
            JsCell::Number(val) => unsafe { f64::to_napi_value(env, val) },
            JsCell::Buffer(bytes) => unsafe { Buffer::to_napi_value(env, bytes.to_vec().into()) },
            JsCell::Date(date) => unsafe { NaiveDateTime::to_napi_value(env, date) },
        }
    }
}
