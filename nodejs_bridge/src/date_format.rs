use crate::error::{BridgeError, ToJsError};
use chrono::{DateTime, Offset, TimeDelta, Utc};
use chrono_tz::Tz;
use napi::Env;
use napi::bindgen_prelude::Either;
use napi_derive::napi;
use sf_core::utils::sync::RwLockRecoverExt;
use sf_output_format::datetime::{DateTimeFormat, DateTimeValue, Zone};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

static COMPILED_FORMATS: LazyLock<RwLock<HashMap<String, Arc<DateTimeFormat>>>> =
    LazyLock::new(Default::default);

pub(crate) fn compiled_format(format: &str) -> Arc<DateTimeFormat> {
    if let Some(compiled) = COMPILED_FORMATS.read_recover().get(format) {
        return Arc::clone(compiled);
    }
    Arc::clone(
        COMPILED_FORMATS
            .write_recover()
            .entry(format.to_owned())
            .or_insert_with(|| Arc::new(DateTimeFormat::compile(format))),
    )
}

fn utc_from_parts(epoch_millis: f64, nanos: u32) -> Result<DateTime<Utc>, BridgeError> {
    let secs = (epoch_millis as i64).div_euclid(1000);
    DateTime::<Utc>::from_timestamp(secs, nanos).ok_or_else(|| {
        BridgeError::Message(format!(
            "timestamp out of range: epoch_millis={epoch_millis}, nanos={nanos}"
        ))
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResolvedTimezone {
    Utc,
    Named(Tz),
    Offset(i32),
}

impl ResolvedTimezone {
    pub(crate) fn from_name(name: &str) -> Result<Self, String> {
        if name.eq_ignore_ascii_case("UTC") || name.eq_ignore_ascii_case("GMT") {
            return Ok(Self::Utc);
        }
        name.parse()
            .map(Self::Named)
            .map_err(|_| format!("unknown timezone: {name}"))
    }

    fn from_js(timezone: Either<String, i32>) -> Result<Self, String> {
        match timezone {
            Either::A(name) => Self::from_name(&name),
            Either::B(offset_minutes) => Ok(Self::Offset(offset_minutes)),
        }
    }

    pub(crate) fn to_js(self) -> Either<String, i32> {
        match self {
            Self::Utc => Either::A("UTC".to_owned()),
            Self::Named(tz) => Either::A(tz.name().to_owned()),
            Self::Offset(offset_minutes) => Either::B(offset_minutes),
        }
    }

    pub(crate) fn format(self, format: &DateTimeFormat, utc: DateTime<Utc>, scale: u32) -> String {
        let (local, offset_minutes, zone) = match self {
            Self::Utc => (utc.naive_utc(), 0, Zone::Named("GMT")),
            Self::Offset(offset_minutes) => (
                utc.naive_utc() + TimeDelta::minutes(i64::from(offset_minutes)),
                offset_minutes,
                Zone::Offset,
            ),
            Self::Named(tz) => {
                let local = utc.with_timezone(&tz);
                (
                    local.naive_local(),
                    local.offset().fix().local_minus_utc() / 60,
                    Zone::Offset,
                )
            }
        };
        format.format(&DateTimeValue {
            local,
            offset_minutes,
            scale,
            zone,
        })
    }
}

#[napi]
pub fn format_snowflake_date(
    env: Env,
    format: String,
    epoch_millis: f64,
    nanos: u32,
    scale: u32,
    timezone: Either<String, i32>,
) -> napi::Result<String> {
    format_date(&format, epoch_millis, nanos, scale, timezone).map_err(|e| e.to_js_error(env, None))
}

fn format_date(
    format: &str,
    epoch_millis: f64,
    nanos: u32,
    scale: u32,
    timezone: Either<String, i32>,
) -> Result<String, BridgeError> {
    let timezone = ResolvedTimezone::from_js(timezone).map_err(BridgeError::Message)?;
    Ok(timezone.format(
        &compiled_format(format),
        utc_from_parts(epoch_millis, nanos)?,
        scale,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn los_angeles() -> ResolvedTimezone {
        ResolvedTimezone::from_name("America/Los_Angeles").unwrap()
    }

    #[test]
    fn from_name_resolves_utc_aliases_named_zones_and_rejects_unknown() {
        assert_eq!(
            ResolvedTimezone::from_name("utc"),
            Ok(ResolvedTimezone::Utc)
        );
        assert_eq!(
            ResolvedTimezone::from_name("GMT"),
            Ok(ResolvedTimezone::Utc)
        );
        assert_eq!(
            los_angeles(),
            ResolvedTimezone::Named(Tz::America__Los_Angeles)
        );
        assert_eq!(
            ResolvedTimezone::from_name("Not/AZone"),
            Err("unknown timezone: Not/AZone".to_string())
        );
    }

    #[test]
    fn js_round_trip_preserves_the_timezone() {
        for timezone in [
            ResolvedTimezone::Utc,
            los_angeles(),
            ResolvedTimezone::Offset(-480),
        ] {
            assert_eq!(ResolvedTimezone::from_js(timezone.to_js()), Ok(timezone));
        }
    }

    #[test]
    fn formats_calendar_date_in_utc() {
        let epoch = DateTime::parse_from_rfc3339("2024-01-15T00:00:00Z")
            .unwrap()
            .timestamp_millis() as f64;
        assert_eq!(
            format_date("YYYY-MM-DD", epoch, 0, 0, Either::A("UTC".into())).unwrap(),
            "2024-01-15"
        );
        assert_eq!(
            format_date("DD-MON-YYYY", epoch, 0, 0, Either::A("UTC".into())).unwrap(),
            "15-Jan-2024"
        );
        assert_eq!(
            format_date("YYYY-MM-DD TZD", epoch, 0, 0, Either::A("UTC".into())).unwrap(),
            "2024-01-15 GMT"
        );
    }

    #[test]
    fn formats_ntz_in_utc() {
        let epoch = DateTime::parse_from_rfc3339("2024-01-15T10:30:00.123456789Z")
            .unwrap()
            .timestamp_millis() as f64;
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF3",
                epoch,
                123_456_789,
                9,
                Either::A("UTC".into())
            )
            .unwrap(),
            "2024-01-15 10:30:00.123"
        );
    }

    #[test]
    fn formats_tz_from_offset_minutes() {
        let epoch = DateTime::parse_from_rfc3339("2024-01-15T05:30:00.123Z")
            .unwrap()
            .timestamp_millis() as f64;
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
                epoch,
                123_000_000,
                9,
                Either::B(300)
            )
            .unwrap(),
            "2024-01-15 10:30:00.123 +0500"
        );
    }

    #[test]
    fn formats_ltz_in_named_zone() {
        let epoch = 1_448_570_571_000.0;
        let zone = Either::A("America/Los_Angeles".into());
        assert_eq!(
            format_date(
                "DY, DD MON YYYY HH24:MI:SS TZHTZM",
                epoch,
                906_000_000,
                9,
                zone.clone()
            )
            .unwrap(),
            "Thu, 26 Nov 2015 12:42:51 -0800"
        );
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF TZHTZM",
                epoch,
                123_456_000,
                6,
                zone.clone()
            )
            .unwrap(),
            "2015-11-26 12:42:51.123456 -0800"
        );
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM",
                epoch,
                123_456_789,
                9,
                zone.clone()
            )
            .unwrap(),
            "2015-11-26 12:42:51.123 -0800"
        );
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF9 TZHTZM",
                epoch,
                123_456_789,
                9,
                zone
            )
            .unwrap(),
            "2015-11-26 12:42:51.123456789 -0800"
        );
    }

    #[test]
    fn formats_negative_epoch_millis() {
        assert_eq!(
            format_date(
                "YYYY-MM-DD HH24:MI:SS.FF3",
                -1_500.0,
                500_000_000,
                3,
                Either::A("UTC".into())
            )
            .unwrap(),
            "1969-12-31 23:59:58.500"
        );
    }

    #[test]
    fn rejects_unknown_timezone_and_out_of_range_nanos() {
        assert!(format_date("YYYY-MM-DD", 0.0, 0, 0, Either::A("Not/AZone".into())).is_err());
        assert!(format_date("YYYY-MM-DD", 0.0, 1_000_000_000, 9, Either::A("UTC".into())).is_err());
    }
}
