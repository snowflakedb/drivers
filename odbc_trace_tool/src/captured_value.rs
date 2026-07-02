//! Typed SQLGetData buffer values captured from live driver replay and
//! persisted in `ir.yaml` for offline C++ assertion generation.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Variant tag strings for [`CapturedValue`]'s serde external-tag encoding.
/// Shared with the C++ emitter so schema drift is caught by unit tests.
pub mod tags {
    pub const BYTES: &str = "Bytes";
    pub const DOUBLE: &str = "Double";
    pub const FLOAT: &str = "Float";
    pub const INT: &str = "Int";
    pub const DATE: &str = "Date";
    pub const TIME: &str = "Time";
    pub const TIMESTAMP: &str = "Timestamp";
}

/// A floating-point payload that round-trips through JSON/YAML without
/// relying on non-finite number encoding (which serde rejects).
#[derive(Debug, Clone, PartialEq)]
pub enum DoubleVal {
    Finite(f64),
    NonFinite(String),
}

impl DoubleVal {
    pub fn from_f64(v: f64) -> Self {
        if v.is_nan() {
            Self::NonFinite("NaN".to_string())
        } else if v.is_infinite() {
            if v.is_sign_negative() {
                Self::NonFinite("-Infinity".to_string())
            } else {
                Self::NonFinite("Infinity".to_string())
            }
        } else {
            Self::Finite(v)
        }
    }
}

impl Serialize for DoubleVal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Finite(v) => serializer.serialize_f64(*v),
            Self::NonFinite(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for DoubleVal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(Self::Finite)
                .ok_or_else(|| serde::de::Error::custom("invalid finite double")),
            serde_json::Value::String(s) => Ok(Self::NonFinite(s)),
            other => Err(serde::de::Error::custom(format!(
                "expected number or string for Double, got {other}"
            ))),
        }
    }
}

/// Same as [`DoubleVal`] but for 4-byte `SQL_C_FLOAT` values.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatVal {
    Finite(f32),
    NonFinite(String),
}

impl FloatVal {
    pub fn from_f32(v: f32) -> Self {
        if v.is_nan() {
            Self::NonFinite("NaN".to_string())
        } else if v.is_infinite() {
            if v.is_sign_negative() {
                Self::NonFinite("-Infinity".to_string())
            } else {
                Self::NonFinite("Infinity".to_string())
            }
        } else {
            Self::Finite(v)
        }
    }
}

impl Serialize for FloatVal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Finite(v) => serializer.serialize_f64(f64::from(*v)),
            Self::NonFinite(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for FloatVal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(|v| Self::Finite(v as f32))
                .ok_or_else(|| serde::de::Error::custom("invalid finite float")),
            serde_json::Value::String(s) => Ok(Self::NonFinite(s)),
            other => Err(serde::de::Error::custom(format!(
                "expected number or string for Float, got {other}"
            ))),
        }
    }
}

/// Decoded buffer value for a single obscured `SQLGetData` call.
#[derive(Debug, Clone, PartialEq)]
pub enum CapturedValue {
    /// Lowercase hex string (no `0x` prefix).
    Bytes(String),
    Double(DoubleVal),
    Float(FloatVal),
    /// Decimal string — JSON numbers are IEEE `double` and cannot hold all
    /// 64-bit integers exactly.
    Int(String),
    Date {
        year: i16,
        month: u16,
        day: u16,
    },
    Time {
        hour: u16,
        minute: u16,
        second: u16,
    },
    Timestamp {
        year: i16,
        month: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        fraction: u32,
    },
}

impl Serialize for CapturedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::Bytes(v) => map.serialize_entry(tags::BYTES, v)?,
            Self::Double(v) => map.serialize_entry(tags::DOUBLE, v)?,
            Self::Float(v) => map.serialize_entry(tags::FLOAT, v)?,
            Self::Int(v) => map.serialize_entry(tags::INT, v)?,
            Self::Date { year, month, day } => map.serialize_entry(
                tags::DATE,
                &DateFields {
                    year: *year,
                    month: *month,
                    day: *day,
                },
            )?,
            Self::Time {
                hour,
                minute,
                second,
            } => map.serialize_entry(
                tags::TIME,
                &TimeFields {
                    hour: *hour,
                    minute: *minute,
                    second: *second,
                },
            )?,
            Self::Timestamp {
                year,
                month,
                day,
                hour,
                minute,
                second,
                fraction,
            } => map.serialize_entry(
                tags::TIMESTAMP,
                &TimestampFields {
                    year: *year,
                    month: *month,
                    day: *day,
                    hour: *hour,
                    minute: *minute,
                    second: *second,
                    fraction: *fraction,
                },
            )?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for CapturedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let map = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("CapturedValue must be a single-key object"))?;
        if map.len() != 1 {
            return Err(serde::de::Error::custom(format!(
                "CapturedValue must be a single-key map, got {map:?}"
            )));
        }
        let (tag, value) = map.iter().next().expect("len checked");
        match tag.as_str() {
            tags::BYTES => {
                let v = String::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Bytes(v))
            }
            tags::DOUBLE => {
                let v = DoubleVal::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Double(v))
            }
            tags::FLOAT => {
                let v = FloatVal::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Float(v))
            }
            tags::INT => {
                let v = String::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Int(v))
            }
            tags::DATE => {
                let f = DateFields::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Date {
                    year: f.year,
                    month: f.month,
                    day: f.day,
                })
            }
            tags::TIME => {
                let f = TimeFields::deserialize(value.clone()).map_err(serde::de::Error::custom)?;
                Ok(Self::Time {
                    hour: f.hour,
                    minute: f.minute,
                    second: f.second,
                })
            }
            tags::TIMESTAMP => {
                let f = TimestampFields::deserialize(value.clone())
                    .map_err(serde::de::Error::custom)?;
                Ok(Self::Timestamp {
                    year: f.year,
                    month: f.month,
                    day: f.day,
                    hour: f.hour,
                    minute: f.minute,
                    second: f.second,
                    fraction: f.fraction,
                })
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown CapturedValue tag {other:?}"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct DateFields {
    year: i16,
    month: u16,
    day: u16,
}

#[derive(Serialize, Deserialize)]
struct TimeFields {
    hour: u16,
    minute: u16,
    second: u16,
}

#[derive(Serialize, Deserialize)]
struct TimestampFields {
    year: i16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    fraction: u32,
}

impl fmt::Display for CapturedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Parse a harness JSON map (`seq` string key -> optional value) into a
/// `HashMap` keyed by sequence number.
pub fn parse_capture_map(
    json: &str,
) -> Result<std::collections::HashMap<u64, CapturedValue>, String> {
    use std::collections::HashMap;

    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("invalid capture JSON: {e}"))?;
    let obj = root
        .as_object()
        .ok_or_else(|| "capture JSON root must be an object".to_string())?;

    let mut by_seq = HashMap::new();
    for (key, val) in obj {
        let seq: u64 = key
            .parse()
            .map_err(|_| format!("invalid seq key {key:?}"))?;
        if val.is_null() {
            continue;
        }
        let captured: CapturedValue = serde_json::from_value(val.clone())
            .map_err(|e| format!("invalid captured value at seq {seq}: {e}"))?;
        by_seq.insert(seq, captured);
    }
    Ok(by_seq)
}

/// Custom serde for `Option<CapturedValue>` so it round-trips inside
/// internally-tagged [`crate::model::OdbcCall`] (tag = "function").
pub mod option {
    use super::CapturedValue;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<CapturedValue>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => serializer.serialize_none(),
            Some(v) => v.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<CapturedValue>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<serde_yaml::Value>::deserialize(deserializer)?;
        match value {
            None | Some(serde_yaml::Value::Null) => Ok(None),
            Some(v) => serde_yaml::from_value(v)
                .map(Some)
                .map_err(serde::de::Error::custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_round_trips_large_value_as_string() {
        let big = "9223372036854775807";
        let cv = CapturedValue::Int(big.to_string());
        let json = serde_json::to_value(&cv).unwrap();
        assert_eq!(json, serde_json::json!({"Int": big}));
        let back: CapturedValue = serde_json::from_value(json).unwrap();
        assert_eq!(back, cv);
    }

    #[test]
    fn double_non_finite_round_trips_as_string() {
        for (val, expected) in [
            (DoubleVal::NonFinite("NaN".into()), "NaN"),
            (DoubleVal::NonFinite("Infinity".into()), "Infinity"),
            (DoubleVal::NonFinite("-Infinity".into()), "-Infinity"),
        ] {
            let cv = CapturedValue::Double(val);
            let json = serde_json::to_value(&cv).unwrap();
            assert_eq!(json, serde_json::json!({"Double": expected}));
            let back: CapturedValue = serde_json::from_value(json).unwrap();
            assert_eq!(back, cv);
        }
    }

    #[test]
    fn float_finite_round_trips() {
        let cv = CapturedValue::Float(FloatVal::Finite(1.5f32));
        let json = serde_json::to_value(&cv).unwrap();
        let back: CapturedValue = serde_json::from_value(json).unwrap();
        assert_eq!(back, cv);
    }

    #[test]
    fn schema_tags_match_constants() {
        let cases = [
            (CapturedValue::Bytes("ab".into()), tags::BYTES),
            (CapturedValue::Double(DoubleVal::Finite(2.5)), tags::DOUBLE),
            (CapturedValue::Float(FloatVal::Finite(1.0)), tags::FLOAT),
            (CapturedValue::Int("42".into()), tags::INT),
            (
                CapturedValue::Date {
                    year: 2024,
                    month: 1,
                    day: 15,
                },
                tags::DATE,
            ),
            (
                CapturedValue::Time {
                    hour: 13,
                    minute: 45,
                    second: 30,
                },
                tags::TIME,
            ),
            (
                CapturedValue::Timestamp {
                    year: 2024,
                    month: 1,
                    day: 15,
                    hour: 13,
                    minute: 45,
                    second: 30,
                    fraction: 0,
                },
                tags::TIMESTAMP,
            ),
        ];
        for (cv, tag) in cases {
            let json = serde_json::to_value(&cv).unwrap();
            let obj = json.as_object().unwrap();
            assert_eq!(obj.len(), 1);
            assert!(obj.contains_key(tag));
        }
    }

    #[test]
    fn getdata_captured_yaml_roundtrip_inside_odbc_call() {
        use crate::model::{GetData, OdbcCall, ReturnCode};

        let call = OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(8),
            target_type_name: Some("SQL_C_DOUBLE".to_string()),
            buffer_length: Some(8),
            value: None,
            indicator: Some(8),
            captured: Some(CapturedValue::Double(DoubleVal::Finite(1.0))),
            seq: None,
        });

        let yaml = serde_yaml::to_string(&call).expect("serialize");
        let back: OdbcCall = serde_yaml::from_str(&yaml).expect("deserialize");
        let OdbcCall::GetData(gd) = back else {
            panic!("expected GetData");
        };
        assert_eq!(
            gd.captured,
            Some(CapturedValue::Double(DoubleVal::Finite(1.0)))
        );
    }

    #[test]
    fn parse_capture_map_skips_nulls() {
        let json = r#"{"42": {"Double": 2.5}, "99": null}"#;
        let map = parse_capture_map(json).unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&42));
    }
}
