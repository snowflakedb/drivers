use chrono::NaiveDateTime;
use napi::bindgen_prelude::{Buffer, Null, Object, ToNapiValue};
use napi::{Env, Result, sys};
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
pub(crate) enum JsCell<'a> {
    Null,
    Bool(bool),
    Str(Cow<'a, str>),
    Number(f64),
    NumberArray(Vec<f64>),
    Buffer(&'a [u8]),
    Date(NaiveDateTime),
    Timestamp {
        epoch_millis: f64,
        offset_minutes: i32,
        nanos: u32,
    },
}

impl<'a> ToNapiValue for JsCell<'a> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        match val {
            JsCell::Null => unsafe { Null::to_napi_value(env, Null) },
            JsCell::Bool(val) => unsafe { bool::to_napi_value(env, val) },
            JsCell::Str(val) => unsafe { ToNapiValue::to_napi_value(env, val.as_ref()) },
            JsCell::Number(val) => unsafe { f64::to_napi_value(env, val) },
            JsCell::NumberArray(vals) => unsafe { Vec::<f64>::to_napi_value(env, vals) },
            JsCell::Buffer(bytes) => unsafe { Buffer::to_napi_value(env, bytes.to_vec().into()) },
            JsCell::Date(date) => unsafe { NaiveDateTime::to_napi_value(env, date) },
            JsCell::Timestamp {
                epoch_millis,
                offset_minutes,
                nanos,
            } => {
                let mut obj = Object::new(&Env::from(env))?;
                obj.set("epochMillis", epoch_millis)?;
                obj.set("offsetMinutes", offset_minutes)?;
                obj.set("nanos", nanos)?;
                unsafe { ToNapiValue::to_napi_value(env, obj) }
            }
        }
    }
}
