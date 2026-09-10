use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyByteArray, PyDate, PyDateAccess, PyDateTime, PyFloat, PyInt, PyString, PyTime,
    PyTimeAccess, PyTzInfoAccess,
};

pub(crate) fn assert_py_bool(value: &Bound<'_, PyAny>, expected: bool) {
    assert!(
        value.is_instance_of::<PyBool>(),
        "expected Python bool, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<bool>().unwrap(), expected);
}

pub(crate) fn assert_py_float(value: &Bound<'_, PyAny>, expected: f64) {
    assert!(
        value.is_instance_of::<PyFloat>(),
        "expected Python float, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<f64>().unwrap(), expected);
}

pub(crate) fn assert_py_str(value: &Bound<'_, PyAny>, expected: &str) {
    assert!(
        value.is_instance_of::<PyString>(),
        "expected Python str, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<String>().unwrap(), expected);
}

pub(crate) fn assert_py_int(value: &Bound<'_, PyAny>, expected: i64) {
    assert!(
        value.is_instance_of::<PyInt>(),
        "expected Python int, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<i64>().unwrap(), expected);
}

pub(crate) fn assert_py_decimal(value: &Bound<'_, PyAny>, expected: &str) {
    assert_eq!(
        value.get_type().name().unwrap(),
        "Decimal",
        "expected decimal.Decimal, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.str().unwrap().to_string(), expected);
}

pub(crate) fn assert_py_bytes(value: &Bound<'_, PyAny>, expected: &[u8]) {
    assert!(
        value.is_instance_of::<PyByteArray>(),
        "expected Python bytearray, got {}",
        value.get_type().name().unwrap()
    );
    assert_eq!(value.extract::<Vec<u8>>().unwrap(), expected);
}

pub(crate) fn assert_py_none(value: &Bound<'_, PyAny>) {
    assert!(value.is_none(), "expected None, got {value}");
}

pub(crate) fn assert_py_date(value: &Bound<'_, PyAny>, year: i32, month: u8, day: u8) {
    assert!(
        value.is_instance_of::<PyDate>(),
        "expected datetime.date, got {}",
        value.get_type().name().unwrap()
    );
    let date = value.cast::<PyDate>().unwrap();
    assert_eq!(
        (date.get_year(), date.get_month(), date.get_day()),
        (year, month, day)
    );
}

pub(crate) fn assert_py_time(
    value: &Bound<'_, PyAny>,
    hour: u8,
    minute: u8,
    second: u8,
    microsecond: u32,
) {
    assert!(
        value.is_instance_of::<PyTime>(),
        "expected datetime.time, got {}",
        value.get_type().name().unwrap()
    );
    let time = value.cast::<PyTime>().unwrap();
    assert_eq!(
        (
            time.get_hour(),
            time.get_minute(),
            time.get_second(),
            time.get_microsecond()
        ),
        (hour, minute, second, microsecond)
    );
}

pub(crate) fn assert_py_datetime(
    value: &Bound<'_, PyAny>,
    date: (i32, u8, u8),
    time: (u8, u8, u8, u32),
) {
    assert!(
        value.is_instance_of::<PyDateTime>(),
        "expected datetime.datetime, got {}",
        value.get_type().name().unwrap()
    );
    let datetime = value.cast::<PyDateTime>().unwrap();
    assert_eq!(
        (
            datetime.get_year(),
            datetime.get_month(),
            datetime.get_day(),
            datetime.get_hour(),
            datetime.get_minute(),
            datetime.get_second(),
            datetime.get_microsecond()
        ),
        (date.0, date.1, date.2, time.0, time.1, time.2, time.3)
    );
    assert!(
        datetime.get_tzinfo().is_none(),
        "expected naive datetime, got tzinfo {:?}",
        datetime.get_tzinfo()
    );
}

pub(crate) fn assert_py_datetime_tz(
    value: &Bound<'_, PyAny>,
    date: (i32, u8, u8),
    time: (u8, u8, u8, u32),
    offset_minutes: i32,
) {
    assert!(
        value.is_instance_of::<PyDateTime>(),
        "expected datetime.datetime, got {}",
        value.get_type().name().unwrap()
    );
    let datetime = value.cast::<PyDateTime>().unwrap();
    assert_eq!(
        (
            datetime.get_year(),
            datetime.get_month(),
            datetime.get_day(),
            datetime.get_hour(),
            datetime.get_minute(),
            datetime.get_second(),
            datetime.get_microsecond()
        ),
        (date.0, date.1, date.2, time.0, time.1, time.2, time.3)
    );
    let tzinfo = datetime
        .get_tzinfo()
        .expect("expected tz-aware datetime, got naive");
    let tz_type = tzinfo.get_type().name().unwrap().to_string();
    assert!(
        tz_type == "timezone" || tz_type == "_FixedOffset",
        "expected datetime.timezone or pytz.FixedOffset, got {tz_type}"
    );
    let offset_total_seconds = datetime
        .call_method0("utcoffset")
        .unwrap()
        .call_method0("total_seconds")
        .unwrap()
        .extract::<f64>()
        .unwrap();
    assert_eq!(
        offset_total_seconds,
        f64::from(offset_minutes) * 60.0,
        "expected offset {offset_minutes} minutes, got {offset_total_seconds} seconds"
    );
}
