//! Snowflake DATE / TIME / TIMESTAMP output formats (`DATE_OUTPUT_FORMAT`
//! and friends, or the format argument of `TO_CHAR`).
//!
//! [`DateTimeFormat::compile`] tokenizes the format string once (longest
//! match, case-insensitive except `UUUU`). Calendar names come from
//! `chrono`; year padding, `FF` width, and timezone spelling are handled
//! here.
//!
//! An unquoted `%` is literal text here. `TO_CHAR` can emit different
//! characters for the same format (for example `%Y` may not print the
//! year). That case is not reproduced.
//!
//! `TZM` on its own is not an element. [`Element::TzAbbrev`] (`TZD`) prints
//! the value's zone designation, which the offset does not determine —
//! callers supply that via [`Zone`].

use std::fmt::Write;

use chrono::{Datelike, NaiveDateTime, Timelike};

const MAX_FRACTION_DIGITS: u32 = 9;

/// A value to render, already shifted into the zone it is displayed in.
///
/// `offset_minutes` is what the timezone elements print, not a shift to
/// apply: DATE, TIME and TIMESTAMP_NTZ pass `0`, TIMESTAMP_TZ passes the
/// value's own offset, TIMESTAMP_LTZ the session zone's offset for that
/// instant.
///
/// `scale` is the column's declared fractional-second precision (0..=9). It
/// is the width a bare `FF` renders; `FFn` overrides it. A wider scale
/// renders no fraction at all, matching [`crate::format_day_time`].
#[derive(Debug, Clone, Copy)]
pub struct DateTimeValue {
    pub local: NaiveDateTime,
    pub offset_minutes: i32,
    pub scale: u32,
    pub zone: Zone,
}

/// What `TZD` prints, which the offset alone does not determine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// A value whose zone is a name rather than an offset. DATE, TIME and
    /// `TIMESTAMP_NTZ` pass `Named("GMT")`. `TIMESTAMP_LTZ` passes the
    /// session zone name.
    Named(&'static str),
    /// A value carrying only an offset, printed as `GMT±hh:mm` including at
    /// offset zero. `TIMESTAMP_TZ` passes this.
    Offset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OffsetStyle {
    /// `TZH:TZM`
    Colon,
    /// `TZHTZM`
    NoColon,
    /// `TZH`, which drops the minutes of a sub-hour offset.
    HourOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Element {
    Literal(String),
    Year4,
    Year2 {
        pad: bool,
    },
    MonthName,
    MonthAbbrev,
    Month {
        pad: bool,
    },
    WeekdayAbbrev,
    Day {
        width: u32,
    },
    Hour24 {
        pad: bool,
    },
    Hour12 {
        pad: bool,
    },
    Meridiem,
    MeridiemInitial,
    Minute {
        pad: bool,
    },
    Second {
        pad: bool,
    },
    /// `None` for a bare `FF`, which takes its width from
    /// [`DateTimeValue::scale`].
    Fraction(Option<u32>),
    TzOffset(OffsetStyle),
    TzAbbrev,
}

impl Element {
    fn render(&self, value: &DateTimeValue, out: &mut Output) {
        let local = value.local;
        match self {
            Element::Literal(literal) => out.push_str(literal),
            Element::Year4 => out.push_year(local.year()),
            Element::Year2 { pad } => out.push_number(local.year().rem_euclid(100) as u32, *pad),
            Element::Month { pad } => out.push_number(local.month(), *pad),
            Element::Day { width } => out.push_width(local.day(), *width),
            Element::Hour24 { pad } => out.push_number(local.hour(), *pad),
            Element::Hour12 { pad } => out.push_number(local.hour12().1, *pad),
            Element::Minute { pad } => out.push_number(local.minute(), *pad),
            Element::Second { pad } => out.push_number(local.second(), *pad),
            Element::MonthAbbrev => out.push_fmt(local.format("%b")),
            Element::MonthName => out.push_fmt(local.format("%B")),
            Element::WeekdayAbbrev => out.push_fmt(local.weekday()),
            Element::Meridiem => out.push_fmt(local.format("%p")),
            Element::MeridiemInitial => out.push_str(if local.hour() < 12 { "A" } else { "P" }),
            Element::Fraction(digits) => {
                out.push_fraction(digits.unwrap_or(value.scale), local.nanosecond())
            }
            Element::TzOffset(style) => out.push_offset(value.offset_minutes, *style),
            Element::TzAbbrev => out.push_zone(value.zone, value.offset_minutes),
        }
    }
}

struct Output {
    buf: String,
}

impl Output {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: String::with_capacity(capacity),
        }
    }

    fn finish(self) -> String {
        self.buf
    }

    fn push(&mut self, c: char) {
        self.buf.push(c);
    }

    fn push_str(&mut self, text: &str) {
        self.buf.push_str(text);
    }

    fn push_fmt(&mut self, value: impl std::fmt::Display) {
        let _ = write!(self.buf, "{value}");
    }

    fn push_padded2(&mut self, value: u32) {
        let _ = write!(self.buf, "{value:02}");
    }

    fn push_number(&mut self, value: u32, pad: bool) {
        if pad {
            self.push_padded2(value);
        } else {
            self.push_fmt(value);
        }
    }

    fn push_width(&mut self, value: u32, width: u32) {
        if width == 0 {
            self.push_fmt(value);
        } else {
            let _ = write!(self.buf, "{value:0width$}", width = width as usize);
        }
    }

    fn push_year(&mut self, year: i32) {
        if year < 0 {
            self.push('-');
            self.push_fmt(year.unsigned_abs());
        } else {
            let _ = write!(self.buf, "{year:04}");
        }
    }

    fn push_fraction(&mut self, digits: u32, nanosecond: u32) {
        if digits == 0 || digits > MAX_FRACTION_DIGITS {
            return;
        }
        let fraction = nanosecond / 10u32.pow(MAX_FRACTION_DIGITS - digits);
        let _ = write!(self.buf, "{fraction:0width$}", width = digits as usize);
    }

    fn push_offset(&mut self, offset_minutes: i32, style: OffsetStyle) {
        if offset_minutes == 0 {
            self.push('Z');
            return;
        }
        self.push_offset_digits(offset_minutes, style);
    }

    fn push_zone(&mut self, zone: Zone, offset_minutes: i32) {
        match zone {
            Zone::Named(name) => self.push_str(name),
            Zone::Offset => {
                self.push_str("GMT");
                self.push_offset_digits(offset_minutes, OffsetStyle::Colon);
            }
        }
    }

    fn push_offset_digits(&mut self, offset_minutes: i32, style: OffsetStyle) {
        let magnitude = offset_minutes.unsigned_abs();
        self.push(if offset_minutes < 0 { '-' } else { '+' });
        self.push_padded2(magnitude / 60);
        match style {
            OffsetStyle::Colon => self.push(':'),
            OffsetStyle::NoColon => {}
            OffsetStyle::HourOnly => return,
        }
        self.push_padded2(magnitude % 60);
    }
}

/// A format string compiled once, for rendering many values.
#[derive(Debug, Clone)]
pub struct DateTimeFormat {
    elements: Vec<Element>,
}

impl DateTimeFormat {
    pub fn compile(sql_format: &str) -> Self {
        let chars: Vec<char> = sql_format.chars().collect();
        let mut elements: Vec<Element> = Vec::new();
        let mut index = 0;

        while index < chars.len() {
            if chars[index] == '"' {
                if chars.get(index + 1) == Some(&'"') {
                    push(&mut elements, Element::Literal("\"".to_string()));
                    index += 2;
                    continue;
                }
                index += 1;
                let mut quoted = String::new();
                while index < chars.len() && chars[index] != '"' {
                    quoted.push(chars[index]);
                    index += 1;
                }
                if index < chars.len() {
                    index += 1;
                }
                push(&mut elements, Element::Literal(quoted));
                continue;
            }

            match match_element(&chars[index..]) {
                Some((length, element)) => {
                    push(&mut elements, element);
                    index += length;
                }
                None => {
                    push(&mut elements, Element::Literal(chars[index].to_string()));
                    index += 1;
                }
            }
        }

        Self { elements }
    }

    pub fn format(&self, value: &DateTimeValue) -> String {
        debug_assert!(value.scale <= MAX_FRACTION_DIGITS);
        let mut out = Output::with_capacity(self.elements.len() * 4);
        for element in &self.elements {
            element.render(value, &mut out);
        }
        out.finish()
    }
}

fn push(elements: &mut Vec<Element>, element: Element) {
    if let (Element::Literal(text), Some(Element::Literal(literal))) =
        (&element, elements.last_mut())
    {
        literal.push_str(text);
        return;
    }
    elements.push(element);
}

/// Order is load-bearing beyond longest-first: `MM` before `MI`/`MON`,
/// `TZH:TZM` before `TZHTZM`.
fn match_element(rest: &[char]) -> Option<(usize, Element)> {
    let candidates: &[(&str, Element)] = match rest[0] {
        'a' | 'A' | 'p' | 'P' => &[
            ("PM", Element::Meridiem),
            ("AM", Element::Meridiem),
            ("P", Element::MeridiemInitial),
        ],
        'd' | 'D' => &[
            ("DDD", Element::Day { width: 3 }),
            ("DD", Element::Day { width: 2 }),
            ("DY", Element::WeekdayAbbrev),
            ("D", Element::Day { width: 0 }),
        ],
        'h' | 'H' => &[
            ("HH24", Element::Hour24 { pad: true }),
            ("HH12", Element::Hour12 { pad: true }),
            ("HH", Element::Hour24 { pad: true }),
            ("H24", Element::Hour24 { pad: false }),
            ("H12", Element::Hour12 { pad: false }),
            ("H", Element::Hour24 { pad: false }),
        ],
        'm' | 'M' => &[
            ("MMMM", Element::MonthName),
            ("MM", Element::Month { pad: true }),
            ("MI", Element::Minute { pad: true }),
            ("MON", Element::MonthAbbrev),
            ("ME", Element::Minute { pad: false }),
            ("MO", Element::Month { pad: false }),
        ],
        's' | 'S' => &[
            ("SS", Element::Second { pad: true }),
            ("S", Element::Second { pad: false }),
        ],
        't' | 'T' => &[
            ("TZD", Element::TzAbbrev),
            ("TZH:TZM", Element::TzOffset(OffsetStyle::Colon)),
            ("TZHTZM", Element::TzOffset(OffsetStyle::NoColon)),
            ("TZHTZH", Element::TzOffset(OffsetStyle::NoColon)),
            ("TZH", Element::TzOffset(OffsetStyle::HourOnly)),
        ],
        // Only an uppercase first letter starts `UUUU`.
        'U' => &[("UUUU", Element::Year4)],
        'y' | 'Y' => &[
            ("YYYY", Element::Year4),
            ("YY", Element::Year2 { pad: true }),
            ("Y", Element::Year2 { pad: false }),
        ],
        'f' | 'F' => {
            if !starts_with_ignore_case(rest, "FF") {
                return None;
            }
            return Some(match rest.get(2).and_then(|c| c.to_digit(10)) {
                Some(digits) => (3, Element::Fraction(Some(digits))),
                None => (2, Element::Fraction(None)),
            });
        }
        _ => &[],
    };

    candidates
        .iter()
        .find(|(token, _)| starts_with_ignore_case(rest, token))
        .map(|(token, element)| (token.len(), element.clone()))
}

fn starts_with_ignore_case(chars: &[char], token: &str) -> bool {
    chars.len() >= token.len()
        && chars
            .iter()
            .zip(token.chars())
            .all(|(c, t)| c.to_ascii_uppercase() == t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn afternoon() -> DateTimeValue {
        DateTimeValue {
            local: NaiveDate::from_ymd_opt(2005, 6, 15)
                .unwrap()
                .and_hms_nano_opt(14, 45, 30, 123_456_789)
                .unwrap(),
            offset_minutes: -(5 * 60 + 30),
            scale: 9,
            zone: Zone::Offset,
        }
    }

    fn single_digit_fields() -> DateTimeValue {
        DateTimeValue {
            local: NaiveDate::from_ymd_opt(2005, 6, 5)
                .unwrap()
                .and_hms_opt(4, 5, 6)
                .unwrap(),
            offset_minutes: 0,
            scale: 0,
            zone: Zone::Named("GMT"),
        }
    }

    fn date(year: i32, month: u32, day: u32) -> DateTimeValue {
        DateTimeValue {
            local: NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            offset_minutes: 0,
            scale: 0,
            zone: Zone::Named("GMT"),
        }
    }

    fn at_time(hour: u32, minute: u32, second: u32, nanos: u32, scale: u32) -> DateTimeValue {
        DateTimeValue {
            local: NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_nano_opt(hour, minute, second, nanos)
                .unwrap(),
            offset_minutes: 0,
            scale,
            zone: Zone::Named("GMT"),
        }
    }

    fn at_offset(offset_minutes: i32) -> DateTimeValue {
        DateTimeValue {
            offset_minutes,
            ..afternoon()
        }
    }

    fn render(format: &str, value: &DateTimeValue) -> String {
        DateTimeFormat::compile(format).format(value)
    }

    fn assert_renders(value: &DateTimeValue, cases: &[(&str, &str)]) {
        for (format, expected) in cases {
            assert_eq!(&render(format, value), expected, "format {format:?}");
        }
    }

    #[test]
    fn every_element_in_its_padded_form() {
        assert_renders(
            &afternoon(),
            &[
                ("YYYY", "2005"),
                ("YY", "05"),
                ("Y", "5"),
                ("UUUU", "2005"),
                ("MM", "06"),
                ("MO", "6"),
                ("MON", "Jun"),
                ("MMMM", "June"),
                ("DDD", "015"),
                ("DD", "15"),
                ("D", "15"),
                ("DY", "Wed"),
                ("HH24", "14"),
                ("H24", "14"),
                ("HH12", "02"),
                ("H12", "2"),
                ("HH", "14"),
                ("H", "14"),
                ("MI", "45"),
                ("ME", "45"),
                ("SS", "30"),
                ("S", "30"),
                ("AM", "PM"),
                ("PM", "PM"),
                ("P", "P"),
                ("FF", "123456789"),
                ("FF3", "123"),
                ("TZH:TZM", "-05:30"),
                ("TZHTZM", "-0530"),
                ("TZH", "-05"),
                ("TZD", "GMT-05:30"),
            ],
        );
    }

    #[test]
    fn no_pad_elements_drop_the_leading_zero_their_padded_twins_keep() {
        assert_renders(
            &single_digit_fields(),
            &[
                ("MM", "06"),
                ("MO", "6"),
                ("DDD", "005"),
                ("DD", "05"),
                ("D", "5"),
                ("HH24", "04"),
                ("H24", "4"),
                ("HH12", "04"),
                ("H12", "4"),
                ("MI", "05"),
                ("ME", "5"),
                ("SS", "06"),
                ("S", "6"),
            ],
        );
    }

    #[test]
    fn composite_formats_render_every_element_in_place() {
        assert_renders(
            &afternoon(),
            &[
                ("YYYY-MM-DD", "2005-06-15"),
                ("YYYY-MM-DD HH24:MI:SS", "2005-06-15 14:45:30"),
                (
                    "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM",
                    "2005-06-15 14:45:30.123 -05:30",
                ),
                ("MMMM DD, YYYY DY", "June 15, 2005 Wed"),
                (
                    "DY, DD MON YYYY HH12:MI:SS AM",
                    "Wed, 15 Jun 2005 02:45:30 PM",
                ),
                ("YYYYMMDDHH24MISS", "20050615144530"),
                ("MM/DD/YYYY", "06/15/2005"),
                ("H12:ME P", "2:45 P"),
            ],
        );
    }

    #[test]
    fn month_names_cover_the_year() {
        let expected = [
            ("Jan", "January"),
            ("Feb", "February"),
            ("Mar", "March"),
            ("Apr", "April"),
            ("May", "May"),
            ("Jun", "June"),
            ("Jul", "July"),
            ("Aug", "August"),
            ("Sep", "September"),
            ("Oct", "October"),
            ("Nov", "November"),
            ("Dec", "December"),
        ];
        for (index, (abbrev, name)) in expected.iter().enumerate() {
            let month = index as u32 + 1;
            let value = date(2024, month, 1);
            assert_eq!(&render("MON", &value), abbrev, "month {month}");
            assert_eq!(&render("MMMM", &value), name, "month {month}");
        }
    }

    #[test]
    fn weekday_abbreviations_cover_the_week() {
        // 2024-01-01 is a Monday, so seven consecutive days name every weekday.
        let expected = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (offset, abbrev) in expected.iter().enumerate() {
            let day = offset as u32 + 1;
            assert_eq!(&render("DY", &date(2024, 1, day)), abbrev, "2024-01-{day}");
        }
    }

    #[test]
    fn four_digit_year_pads_but_never_truncates() {
        for (year, expected) in [
            (1, "0001"),
            (9, "0009"),
            (10, "0010"),
            (99, "0099"),
            (100, "0100"),
            (999, "0999"),
            (1000, "1000"),
            (9999, "9999"),
        ] {
            assert_eq!(&render("YYYY", &date(year, 1, 1)), expected, "year {year}");
        }
    }

    #[test]
    fn two_digit_year_keeps_the_last_two_digits() {
        for (year, padded, unpadded) in [
            (1, "01", "1"),
            (1970, "70", "70"),
            (2000, "00", "0"),
            (2005, "05", "5"),
            (2015, "15", "15"),
            (9999, "99", "99"),
        ] {
            let value = date(year, 1, 1);
            assert_eq!(&render("YY", &value), padded, "YY of {year}");
            assert_eq!(&render("Y", &value), unpadded, "Y of {year}");
        }
    }

    #[test]
    fn the_year_alias_renders_the_same_digits_as_the_four_digit_year() {
        for year in [1, 999, 2005, 9999] {
            let value = date(year, 1, 1);
            assert_eq!(
                render("UUUU", &value),
                render("YYYY", &value),
                "year {year}"
            );
        }
    }

    #[test]
    fn negative_year_prints_its_digits_unpadded_after_the_sign() {
        // Outside Snowflake's DATE range; pinned because the element is
        // defined for the whole chrono year range.
        assert_eq!(render("YYYY", &date(-5, 1, 1)), "-5");
    }

    #[test]
    fn explicit_fraction_width_wins_over_the_scale_in_both_directions() {
        let value = at_time(12, 0, 0, 123_456_789, 9);
        for (format, expected) in [
            ("FF0", ""),
            ("FF1", "1"),
            ("FF2", "12"),
            ("FF3", "123"),
            ("FF4", "1234"),
            ("FF5", "12345"),
            ("FF6", "123456"),
            ("FF7", "1234567"),
            ("FF8", "12345678"),
            ("FF9", "123456789"),
        ] {
            assert_eq!(&render(format, &value), expected, "format {format}");
        }

        assert_eq!(render("FF3", &at_time(12, 0, 0, 0, 0)), "000");
        assert_eq!(render("FF9", &at_time(12, 0, 0, 0, 0)), "000000000");
    }

    #[test]
    fn bare_fraction_takes_its_width_from_the_scale() {
        for (scale, nanos, expected) in [
            (0, 0, ""),
            (1, 500_000_000, "5"),
            (3, 123_000_000, "123"),
            (6, 123_456_000, "123456"),
            (9, 123_456_789, "123456789"),
        ] {
            assert_eq!(
                &render("FF", &at_time(12, 0, 0, nanos, scale)),
                expected,
                "scale {scale}"
            );
        }
    }

    #[test]
    fn a_digit_less_fraction_leaves_the_separator_behind() {
        assert_eq!(
            render("HH24:MI:SS.FF", &at_time(12, 0, 0, 0, 0)),
            "12:00:00."
        );
        assert_eq!(
            render("HH24:MI:SS.FF0", &at_time(12, 0, 0, 0, 9)),
            "12:00:00."
        );
        assert_eq!(
            render("HH24:MI:SS,FF", &at_time(12, 0, 0, 0, 0)),
            "12:00:00,"
        );
    }

    #[test]
    fn fraction_truncates_rather_than_rounds() {
        let value = at_time(12, 0, 0, 999_999_999, 9);
        assert_eq!(render("FF1", &value), "9");
        assert_eq!(render("FF8", &value), "99999999");
    }

    #[test]
    fn hour12_wraps_the_clock_and_the_meridiem_follows_it() {
        for (hour, hour12, meridiem, initial) in [
            (0, "12", "AM", "A"),
            (1, "01", "AM", "A"),
            (11, "11", "AM", "A"),
            (12, "12", "PM", "P"),
            (13, "01", "PM", "P"),
            (23, "11", "PM", "P"),
        ] {
            let value = at_time(hour, 0, 0, 0, 0);
            assert_eq!(&render("HH12", &value), hour12, "hour {hour}");
            assert_eq!(&render("AM", &value), meridiem, "hour {hour}");
            assert_eq!(&render("PM", &value), meridiem, "hour {hour}");
            assert_eq!(&render("P", &value), initial, "hour {hour}");
        }
    }

    #[test]
    fn hour24_and_its_aliases_do_not_wrap() {
        for (hour, expected) in [(0, "00"), (12, "12"), (13, "13"), (23, "23")] {
            let value = at_time(hour, 0, 0, 0, 0);
            assert_eq!(&render("HH24", &value), expected, "hour {hour}");
            assert_eq!(&render("HH", &value), expected, "hour {hour}");
        }
    }

    #[test]
    fn zero_offset_renders_z_rather_than_digits() {
        assert_renders(
            &at_offset(0),
            &[("TZH:TZM", "Z"), ("TZHTZM", "Z"), ("TZH", "Z")],
        );
    }

    #[test]
    fn an_offset_with_zero_hours_renders_digits_not_z() {
        assert_renders(
            &at_offset(30),
            &[("TZH:TZM", "+00:30"), ("TZHTZM", "+0030"), ("TZH", "+00")],
        );
        assert_renders(
            &at_offset(-30),
            &[("TZH:TZM", "-00:30"), ("TZHTZM", "-0030"), ("TZH", "-00")],
        );
    }

    #[test]
    fn the_zone_element_prints_the_designation_not_the_offset() {
        assert_renders(&date(2024, 1, 15), &[("TZD", "GMT")]);
        assert_renders(&at_offset(0), &[("TZD", "GMT+00:00")]);
        assert_renders(&at_offset(5 * 60), &[("TZD", "GMT+05:00")]);
        assert_renders(
            &DateTimeValue {
                zone: Zone::Named("GMT"),
                ..at_offset(0)
            },
            &[("TZD", "GMT")],
        );
    }

    #[test]
    fn offset_elements_render_sign_hours_and_minutes() {
        for (offset_minutes, colon, no_colon, hour_only, abbrev) in [
            (5 * 60, "+05:00", "+0500", "+05", "GMT+05:00"),
            (-(5 * 60), "-05:00", "-0500", "-05", "GMT-05:00"),
            (5 * 60 + 45, "+05:45", "+0545", "+05", "GMT+05:45"),
            (12 * 60 + 45, "+12:45", "+1245", "+12", "GMT+12:45"),
            (14 * 60, "+14:00", "+1400", "+14", "GMT+14:00"),
            (-(12 * 60), "-12:00", "-1200", "-12", "GMT-12:00"),
            (-30, "-00:30", "-0030", "-00", "GMT-00:30"),
        ] {
            let value = at_offset(offset_minutes);
            assert_eq!(&render("TZH:TZM", &value), colon, "offset {offset_minutes}");
            assert_eq!(
                &render("TZHTZM", &value),
                no_colon,
                "offset {offset_minutes}"
            );
            assert_eq!(&render("TZH", &value), hour_only, "offset {offset_minutes}");
            assert_eq!(&render("TZD", &value), abbrev, "offset {offset_minutes}");
        }
    }

    #[test]
    fn every_element_matches_lowercase_and_mixed_case_spelling() {
        let value = afternoon();
        for (format, expected) in [
            ("yyyy-mm-dd hh24:mi:ss", "2005-06-15 14:45:30"),
            ("YyYy-Mm-Dd Hh12:Mi:Ss aM", "2005-06-15 02:45:30 PM"),
            ("mon mmmm dy", "Jun June Wed"),
            ("ff3", "123"),
            ("tzh:tzm tzhtzm tzh tzd", "-05:30 -0530 -05 GMT-05:30"),
            ("mo/d/yy h24:me:s p", "6/15/05 14:45:30 P"),
        ] {
            assert_eq!(&render(format, &value), expected, "format {format:?}");
        }
    }

    #[test]
    fn the_year_alias_is_the_only_case_sensitive_spelling() {
        assert_renders(
            &afternoon(),
            &[
                ("UUUU", "2005"),
                ("Uuuu", "2005"),
                ("UuUu", "2005"),
                ("uuuu", "uuuu"),
                ("uUUU", "uUUU"),
            ],
        );
    }

    #[test]
    fn longer_tokens_win_over_the_shorter_ones_they_start_with() {
        let value = afternoon();
        assert_renders(
            &value,
            &[
                ("HH24", "14"),
                ("HH12", "02"),
                ("HH", "14"),
                ("H24", "14"),
                ("MMMM", "June"),
                ("MM", "06"),
                ("MON", "Jun"),
                // A third M has no token left to join, so it stays literal.
                ("MMM", "06M"),
                ("MMMMM", "JuneM"),
                ("TZH:TZM", "-05:30"),
                ("TZHTZM", "-0530"),
                ("TZHTZH", "-0530"),
                // A stray third Y is its own one-digit year, not leftover text.
                ("YYYY", "2005"),
                ("YYY", "055"),
            ],
        );
    }

    #[test]
    fn a_digit_after_ffn_is_literal_text() {
        assert_eq!(render("FF10", &afternoon()), "10");
        assert_eq!(render("FF3X", &afternoon()), "123X");
    }

    #[test]
    fn unquoted_prose_is_consumed_by_the_elements_its_letters_spell() {
        assert_renders(
            &afternoon(),
            &[("March", "Marc14"), ("seconds", "30econ1530")],
        );
    }

    #[test]
    fn characters_no_element_starts_with_pass_through_unchanged() {
        let value = afternoon();
        assert_renders(
            &value,
            &[
                ("", ""),
                (
                    "[](){}<>/-:.,;|+*=?!'@#$^&~ ",
                    "[](){}<>/-:.,;|+*=?!'@#$^&~ ",
                ),
                ("YYYY[QQ]DD", "2005[QQ]15"),
                ("%", "%"),
            ],
        );
    }

    #[test]
    fn undefined_spellings_decompose_into_the_elements_they_contain() {
        assert_renders(
            &afternoon(),
            &[
                ("DAY", "15A5"),
                ("MONTH", "JunT14"),
                ("SYYYY", "302005"),
                ("TZM", "TZM"),
                ("TZISO", "TZI30O"),
                ("%Y", "%5"),
            ],
        );
    }

    #[test]
    fn double_quoted_text_is_literal_and_the_quotes_are_dropped() {
        let value = afternoon();
        assert_renders(
            &value,
            &[
                (r#""Year:" YYYY"#, "Year: 2005"),
                (r#"YYYY"-"MM"-"DD"#, "2005-06-15"),
                (r#""YYYY MM DD""#, "YYYY MM DD"),
                (r#"YYYY"年"MM"月"DD"日""#, "2005年06月15日"),
                (r#"YYYY""MM"#, r#"2005"06"#),
                (r#"YYYY"unterminated"#, "2005unterminated"),
            ],
        );
    }

    #[test]
    fn a_compiled_format_renders_many_values() {
        let format = DateTimeFormat::compile("YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM");
        assert_eq!(
            format.format(&afternoon()),
            "2005-06-15 14:45:30.123 -05:30"
        );
        assert_eq!(
            format.format(&date(2024, 1, 15)),
            "2024-01-15 00:00:00.000 Z"
        );
        assert_eq!(
            format.format(&at_offset(3 * 60)),
            "2005-06-15 14:45:30.123 +03:00"
        );
    }

    #[test]
    fn a_date_renders_midnight_and_no_offset_for_the_elements_it_has_no_value_for() {
        assert_renders(
            &date(2024, 1, 15),
            &[
                ("YYYY-MM-DD HH24:MI:SS", "2024-01-15 00:00:00"),
                ("HH12:MI:SS AM", "12:00:00 AM"),
                ("HH24:MI:SS.FF", "00:00:00."),
                ("HH24:MI:SS TZH:TZM", "00:00:00 Z"),
                ("TZD", "GMT"),
            ],
        );
    }

    #[test]
    fn a_time_renders_the_unix_epoch_for_the_date_elements_it_has_no_value_for() {
        assert_renders(
            &at_time(14, 45, 30, 0, 0),
            &[
                ("YYYY-MM-DD HH24:MI:SS", "1970-01-01 14:45:30"),
                ("DY MON YY", "Thu Jan 70"),
                ("MMMM", "January"),
                ("HH24:MI:SS TZH:TZM", "14:45:30 Z"),
            ],
        );
    }
}
