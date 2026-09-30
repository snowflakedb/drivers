use chrono::{DateTime, Utc};
use napi_derive::napi;
use sf_output_format::datetime::{DateTimeFormat, DateTimeValue, Zone};

#[napi]
pub struct DateFormatter {
    format: DateTimeFormat,
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
    pub fn format(&self, date: DateTime<Utc>) -> String {
        self.format.format(&DateTimeValue {
            local: date.naive_utc(),
            offset_minutes: 0,
            scale: 0,
            zone: Zone::Named("GMT"),
        })
    }
}
