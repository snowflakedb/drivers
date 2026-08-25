use chrono::{NaiveTime, Timelike};

/// Renders a decoded TIME value the way Snowflake's `TIME_OUTPUT_FORMAT`
/// would, for the token subset this driver supports:
///
/// - `HH24` — 24-hour, zero-padded.
/// - `HH12` — 12-hour, zero-padded, with real wraparound (`chrono`'s
///   [`Timelike::hour12`] already gives `0`/`12` -> `12`, `13` -> `1`, etc).
///   No meridian (`AM`/`PM`) token is supported.
/// - Bare `HH` — an alias for `HH24`, checked only after `HH24`/`HH12` both
///   fail so the longer tokens win. Undocumented on Snowflake's public
///   format-token page, but JDBC and the Python connector both alias it
///   the same way — a deliberate cross-driver convention, not dead grammar.
/// - `MI` / `SS` — minute/second, zero-padded.
/// - Bare `FF`, or `FF0`-`FF9` — fractional seconds. A column's declared
///   `scale` is the number of *real* stored digits; requesting more via
///   `FFn` doesn't invent digits (no rounding/padding with noise), and
///   requesting fewer truncates. When the resolved digit count is zero
///   (e.g. `FF9` against a scale-0 column), this renders nothing and also
///   drops an immediately-preceding literal `.` — the legacy driver leaves
///   a dangling `.` in this case; that's a documented quirk, not a
///   contract, and isn't reproduced here.
///
/// `TZH`/`TZM`/timezone tokens are not recognized — TIME is timezone-
/// agnostic by design, so any such token (or any other unrecognized
/// character) passes through the output unchanged.
pub(crate) fn render(time: NaiveTime, scale: u32, format: &str) -> String {
    let chars: Vec<char> = format.chars().collect();

    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        match match_token(&chars[i..], time, scale) {
            Some((token_len, rendered)) => {
                if rendered.is_empty() && out.ends_with('.') {
                    out.pop();
                }
                out.push_str(&rendered);
                i += token_len;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Tries every recognized token against the start of `rest`, longest match
/// first so e.g. `HH24` doesn't fall through to the bare `HH` arm, and
/// `FF3` doesn't fall through to bare `FF`. Returns the token's length (in
/// `chars`, matching `rest`'s indexing) and its rendered replacement.
fn match_token(rest: &[char], time: NaiveTime, scale: u32) -> Option<(usize, String)> {
    if starts_with(rest, "HH24") {
        return Some((4, format!("{:02}", time.hour())));
    }
    if starts_with(rest, "HH12") {
        let (_is_pm, hour12) = time.hour12();
        return Some((4, format!("{:02}", hour12)));
    }
    // Bare HH: alias for HH24, matched only after HH24/HH12 fail (see
    // module doc comment).
    if starts_with(rest, "HH") {
        return Some((2, format!("{:02}", time.hour())));
    }
    if rest.len() >= 3 && rest[0] == 'F' && rest[1] == 'F' && rest[2].is_ascii_digit() {
        let requested = rest[2].to_digit(10).expect("guarded by is_ascii_digit");
        return Some((3, render_fraction(time, scale, requested)));
    }
    if starts_with(rest, "MI") {
        return Some((2, format!("{:02}", time.minute())));
    }
    if starts_with(rest, "SS") {
        return Some((2, format!("{:02}", time.second())));
    }
    if starts_with(rest, "FF") {
        // Bare FF: render every real digit the column's scale carries.
        return Some((2, render_fraction(time, scale, scale)));
    }
    None
}

fn starts_with(chars: &[char], token: &str) -> bool {
    let token_len = token.chars().count();
    chars.len() >= token_len
        && chars[..token_len]
            .iter()
            .eq(token.chars().collect::<Vec<_>>().iter())
}

/// Renders up to `requested_digits` of the column's real fractional-second
/// digits (never more than `scale` actually carries), zero-padded, then
/// truncated (not rounded) to the requested count.
fn render_fraction(time: NaiveTime, scale: u32, requested_digits: u32) -> String {
    let digits = requested_digits.min(scale);
    if digits == 0 {
        return String::new();
    }
    // `time.nanosecond()` was built as `frac * 10^(9 - scale)` (see
    // `column_reader.rs`'s TIME arm) — dividing back out recovers `frac`,
    // the actual `scale`-digit stored value, with no precision invented.
    let frac = time.nanosecond() / 10u32.pow(9 - scale);
    let full = format!("{:0width$}", frac, width = scale as usize);
    full[..digits as usize].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(h: u32, m: u32, s: u32, nanos: u32) -> NaiveTime {
        NaiveTime::from_hms_nano_opt(h, m, s, nanos).unwrap()
    }

    #[test]
    fn default_format_drops_fractional_seconds() {
        assert_eq!(
            render(time(12, 34, 56, 789_789_789), 9, "HH24:MI:SS"),
            "12:34:56"
        );
    }

    #[test]
    fn bare_ff_renders_all_real_digits_at_full_scale() {
        assert_eq!(
            render(time(12, 34, 56, 789_789_789), 9, "HH24:MI:SS.FF"),
            "12:34:56.789789789"
        );
    }

    #[test]
    fn ff3_truncates_to_three_digits() {
        assert_eq!(
            render(time(12, 34, 56, 789_789_789), 9, "HH24:MI:SS.FF3"),
            "12:34:56.789"
        );
    }

    #[test]
    fn ff9_against_scale_zero_column_drops_digits_and_the_dot() {
        // The column itself is scale 0 (e.g. `TIME(0)`) — nothing was ever
        // stored below whole seconds, so requesting FF9 must not invent
        // digits, and the legacy driver's dangling "12:34:56." is
        // deliberately not reproduced (Design decision 6).
        assert_eq!(render(time(12, 34, 56, 0), 0, "HH24:MI:SS.FF9"), "12:34:56");
    }

    #[test]
    fn bare_ff_against_scale_zero_column_drops_digits_and_the_dot() {
        // Mirrors `ff9_against_scale_zero_column_drops_digits_and_the_dot`,
        // but exercises the bare-`FF` fallback arm in `match_token` (no
        // digit suffix) rather than the `FFn` arm — both call
        // `render_fraction`, but only a dedicated test proves the bare
        // token also drops the digits and the dangling `.` at scale 0.
        assert_eq!(render(time(12, 34, 56, 0), 0, "HH24:MI:SS.FF"), "12:34:56");
    }

    #[test]
    fn hh12_wraps_to_twelve_hour_clock_not_aliased_to_hh24() {
        // Noon: HH12 and HH24 coincide (both "12") — not a distinguishing
        // case on its own. These two do distinguish real 12-hour wraparound.
        assert_eq!(render(time(8, 15, 30, 0), 0, "HH12:MI:SS"), "08:15:30");
        assert_eq!(render(time(14, 45, 30, 0), 0, "HH12:MI:SS"), "02:45:30");
    }

    #[test]
    fn bare_hh_aliases_to_hh24_not_hh12() {
        // 14:45:30 distinguishes HH24 from HH12 (which would wrap to
        // "02"), same reasoning as the HH12 test above.
        assert_eq!(render(time(14, 45, 30, 0), 0, "HH:MI:SS"), "14:45:30");
    }

    #[test]
    fn midnight_hh24_is_zero_not_twelve() {
        assert_eq!(render(time(0, 0, 0, 0), 0, "HH24:MI:SS"), "00:00:00");
    }

    #[test]
    fn unrecognized_tokens_pass_through_unchanged() {
        // TIME is timezone-agnostic — a TZH:TZM token in the format string
        // (copied from a TIMESTAMP_TZ session setting, say) is not
        // substituted, by design.
        assert_eq!(
            render(time(12, 34, 56, 0), 0, "HH24:MI:SS TZH:TZM"),
            "12:34:56 TZH:TZM"
        );
    }
}
