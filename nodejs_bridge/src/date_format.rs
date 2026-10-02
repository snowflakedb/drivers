use chrono::{DateTime, NaiveDateTime, Offset, TimeDelta, Utc};
use chrono_tz::Tz;
use napi::bindgen_prelude::Either;
use napi_derive::napi;
use sf_output_format::datetime::{DateTimeFormat, DateTimeValue, Zone};

#[napi]
pub struct DateFormatter {
    format: DateTimeFormat,
}

fn utc_from_parts(epoch_millis: f64, nanos: u32) -> napi::Result<DateTime<Utc>> {
    let secs = (epoch_millis as i64).div_euclid(1000);
    DateTime::<Utc>::from_timestamp(secs, nanos).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "timestamp out of range: epoch_millis={epoch_millis}, nanos={nanos}"
        ))
    })
}

fn local_in_timezone(
    utc: DateTime<Utc>,
    timezone: Either<String, i32>,
) -> napi::Result<(NaiveDateTime, i32, Zone)> {
    match timezone {
        Either::B(offset_minutes) => Ok((
            utc.naive_utc() + TimeDelta::minutes(i64::from(offset_minutes)),
            offset_minutes,
            Zone::Offset,
        )),
        Either::A(name) if name.eq_ignore_ascii_case("UTC") || name.eq_ignore_ascii_case("GMT") => {
            Ok((utc.naive_utc(), 0, Zone::Named("GMT")))
        }
        Either::A(name) => {
            let tz: Tz = name
                .parse()
                .map_err(|_| napi::Error::from_reason(format!("unknown timezone: {name}")))?;
            let local = utc.with_timezone(&tz);
            Ok((
                local.naive_local(),
                local.offset().fix().local_minus_utc() / 60,
                Zone::Offset,
            ))
        }
    }
}

#[napi]
impl DateFormatter {
    #[napi(constructor)]
    pub fn new(format: String) -> Self {
        Self {
            format: DateTimeFormat::compile(&format),
        }
    }

    #[napi]
    pub fn format(
        &self,
        epoch_millis: f64,
        nanos: u32,
        scale: u32,
        timezone: Either<String, i32>,
    ) -> napi::Result<String> {
        let (local, offset_minutes, zone) =
            local_in_timezone(utc_from_parts(epoch_millis, nanos)?, timezone)?;
        Ok(self.format.format(&DateTimeValue {
            local,
            offset_minutes,
            scale,
            zone,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatter(format: &str) -> DateFormatter {
        DateFormatter::new(format.to_string())
    }

    #[test]
    fn formats_calendar_date_in_utc() {
        let epoch = DateTime::parse_from_rfc3339("2024-01-15T00:00:00Z")
            .unwrap()
            .timestamp_millis() as f64;
        assert_eq!(
            formatter("YYYY-MM-DD")
                .format(epoch, 0, 0, Either::A("UTC".into()))
                .unwrap(),
            "2024-01-15"
        );
        assert_eq!(
            formatter("DD-MON-YYYY")
                .format(epoch, 0, 0, Either::A("UTC".into()))
                .unwrap(),
            "15-Jan-2024"
        );
        assert_eq!(
            formatter("YYYY-MM-DD TZD")
                .format(epoch, 0, 0, Either::A("UTC".into()))
                .unwrap(),
            "2024-01-15 GMT"
        );
    }

    #[test]
    fn formats_ntz_in_utc() {
        let epoch = DateTime::parse_from_rfc3339("2024-01-15T10:30:00.123456789Z")
            .unwrap()
            .timestamp_millis() as f64;
        assert_eq!(
            formatter("YYYY-MM-DD HH24:MI:SS.FF3")
                .format(epoch, 123_456_789, 9, Either::A("UTC".into()))
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
            formatter("YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM")
                .format(epoch, 123_000_000, 9, Either::B(300))
                .unwrap(),
            "2024-01-15 10:30:00.123 +0500"
        );
    }

    #[test]
    fn formats_ltz_in_named_zone() {
        let epoch = 1_448_570_571_000.0;
        let zone = Either::A("America/Los_Angeles".into());
        assert_eq!(
            formatter("DY, DD MON YYYY HH24:MI:SS TZHTZM")
                .format(epoch, 906_000_000, 9, zone.clone())
                .unwrap(),
            "Thu, 26 Nov 2015 12:42:51 -0800"
        );
        assert_eq!(
            formatter("YYYY-MM-DD HH24:MI:SS.FF TZHTZM")
                .format(epoch, 123_456_000, 6, zone.clone())
                .unwrap(),
            "2015-11-26 12:42:51.123456 -0800"
        );
        assert_eq!(
            formatter("YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM")
                .format(epoch, 123_456_789, 9, zone.clone())
                .unwrap(),
            "2015-11-26 12:42:51.123 -0800"
        );
        assert_eq!(
            formatter("YYYY-MM-DD HH24:MI:SS.FF9 TZHTZM")
                .format(epoch, 123_456_789, 9, zone)
                .unwrap(),
            "2015-11-26 12:42:51.123456789 -0800"
        );
    }

    #[test]
    fn formats_negative_epoch_millis() {
        assert_eq!(
            formatter("YYYY-MM-DD HH24:MI:SS.FF3")
                .format(-1_500.0, 500_000_000, 3, Either::A("UTC".into()))
                .unwrap(),
            "1969-12-31 23:59:58.500"
        );
    }

    #[test]
    fn rejects_unknown_timezone_and_out_of_range_nanos() {
        let formatter = formatter("YYYY-MM-DD");
        assert!(
            formatter
                .format(0.0, 0, 0, Either::A("Not/AZone".into()))
                .is_err()
        );
        assert!(
            formatter
                .format(0.0, 1_000_000_000, 9, Either::A("UTC".into()))
                .is_err()
        );
    }
}
