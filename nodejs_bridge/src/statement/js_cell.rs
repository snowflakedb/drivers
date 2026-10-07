use napi::bindgen_prelude::{Buffer, Either, Null, ToNapiValue};
use napi::{Env, Result, sys};
use std::borrow::Cow;

use crate::class_constructors::ClassConstructors;

#[derive(Debug, PartialEq)]
pub(crate) enum DateTimezone<'a> {
    Named(&'a str),
    Offset(i32),
}

#[derive(Debug, PartialEq)]
pub(crate) enum JsCell<'a> {
    Null,
    Bool(bool),
    Str(Cow<'a, str>),
    Number(f64),
    NumberArray(Vec<f64>),
    Buffer(&'a [u8]),
    SnowflakeDate {
        epoch_millis: f64,
        nanos: u32,
        scale: u32,
        timezone: DateTimezone<'a>,
        format: &'a str,
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
            JsCell::SnowflakeDate {
                epoch_millis,
                nanos,
                scale,
                timezone,
                format,
            } => {
                let env = Env::from_raw(env);
                let timezone = match timezone {
                    DateTimezone::Named(name) => Either::A(name.to_owned()),
                    DateTimezone::Offset(minutes) => Either::B(minutes),
                };
                let date = ClassConstructors::get(&env)?
                    .snowflake_date
                    .borrow_back(&env)?
                    .new_instance(
                        (epoch_millis, nanos, scale, timezone, format.to_owned()).into(),
                    )?;
                unsafe { ToNapiValue::to_napi_value(env.raw(), date) }
            }
        }
    }
}
