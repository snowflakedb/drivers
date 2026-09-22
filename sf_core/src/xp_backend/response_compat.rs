//! Tolerances the query-response deserializer needs when a host relays the body
//! instead of the driver reading it off the wire. Lives here so the only trace of
//! the backend in [`crate::rest::snowflake::query_response`] is a
//! `deserialize_with` attribute per affected field.

use serde::Deserialize;

/// Accepts a JSON number whether it arrives as an integer or a float. GS sends
/// `queryContext.entries[].timestamp` as an integer; XP relays the same
/// microsecond timestamp as `1787728200656950.0`, which the strict `i64` rejected.
///
/// Applied on every query response, including HTTP, because the type is shared.
/// Integers are tried first so values beyond 2^53 stay exact when JSON encodes
/// them as integers.
///
/// `query_context_cache` keys and evicts on these fields and sends them on later
/// queries. A float converts with `as i64` (truncates toward zero, saturates out
/// of range). XP sends whole-number floats; values that are not finite are
/// rejected. Floats whose magnitude exceeds 2^53 cannot be recovered exactly.
pub(crate) fn deserialize_lenient_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrFloat {
        Int(i64),
        Float(f64),
    }

    match IntOrFloat::deserialize(deserializer)? {
        IntOrFloat::Int(value) => Ok(value),
        IntOrFloat::Float(value) => {
            if !value.is_finite() {
                return Err(serde::de::Error::custom(
                    "query context numeric field must be finite",
                ));
            }
            Ok(value as i64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::snowflake::query_response::Response;

    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(deserialize_with = "deserialize_lenient_i64")]
        value: i64,
    }

    fn parse(json: &str) -> i64 {
        serde_json::from_str::<Wrapper>(json)
            .expect("should parse")
            .value
    }

    #[test]
    fn accepts_an_integer() {
        assert_eq!(parse(r#"{"value": 1681400000}"#), 1681400000);
        assert_eq!(parse(r#"{"value": -7}"#), -7);
    }

    #[test]
    fn accepts_a_float() {
        assert_eq!(parse(r#"{"value": 1787728200656950.0}"#), 1787728200656950);
        assert_eq!(parse(r#"{"value": 0.0}"#), 0);
    }

    #[test]
    fn a_non_integer_float_truncates_toward_zero() {
        assert_eq!(parse(r#"{"value": 1.9}"#), 1);
        assert_eq!(parse(r#"{"value": -1.9}"#), -1);
    }

    /// 2^53+1 is not representable as f64, so this returns 2^53 if the variant
    /// order is reversed.
    #[test]
    fn an_integer_beyond_2_pow_53_is_not_widened_to_float() {
        assert_eq!(parse(r#"{"value": 9007199254740993}"#), 9007199254740993);
    }

    #[test]
    fn rejects_a_non_number() {
        assert!(serde_json::from_str::<Wrapper>(r#"{"value": "1681400000"}"#).is_err());
        assert!(serde_json::from_str::<Wrapper>(r#"{"value": null}"#).is_err());
    }

    /// Covers the `deserialize_with` attributes on `QueryContextEntry`, not just
    /// this function in isolation.
    #[test]
    fn a_query_response_carrying_float_query_context_parses() {
        let json = r#"{
            "data": {
                "queryContext": {
                    "entries": [
                        {"id": 3575747553.0, "timestamp": 1787728200656950.0, "priority": 0.0, "context": "ctx"}
                    ]
                }
            },
            "success": true
        }"#;

        let response: Response = serde_json::from_str(json).expect("should parse");
        assert!(response.success);
        let entries = response
            .data
            .query_context
            .as_ref()
            .and_then(|ctx| ctx.entries.as_ref())
            .expect("query context entries");
        assert_eq!(entries[0].id, 3575747553);
        assert_eq!(entries[0].timestamp, 1787728200656950);
        assert_eq!(entries[0].priority, 0);
    }
}
