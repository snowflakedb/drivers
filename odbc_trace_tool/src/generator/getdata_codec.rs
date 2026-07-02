//! Shared SQLGetData value codec for capture recording and assertion emission.

use crate::captured_value::{tags, CapturedValue, DoubleVal, FloatVal};

/// Target C types whose buffer contents are not available from trace logs.
pub fn is_obscured_target(target_type: &str) -> bool {
    !matches!(
        target_type,
        "SQL_C_CHAR" | "SQL_CHAR" | "SQL_C_WCHAR" | "SQL_WCHAR"
    )
}

/// C++ integer literal suffix/type for width-correct assertions.
pub fn integer_cpp_type(target_type: &str) -> Option<&'static str> {
    match target_type {
        "SQL_C_TINYINT" | "SQL_C_STINYINT" => Some("SQLSCHAR"),
        "SQL_C_UTINYINT" => Some("SQLCHAR"),
        "SQL_C_SHORT" | "SQL_C_SSHORT" => Some("SQLSMALLINT"),
        "SQL_C_USHORT" => Some("SQLUSMALLINT"),
        "SQL_C_LONG" | "SQL_C_SLONG" => Some("SQLINTEGER"),
        "SQL_C_ULONG" => Some("SQLUINTEGER"),
        "SQL_C_SBIGINT" => Some("SQLBIGINT"),
        "SQL_C_UBIGINT" => Some("SQLUBIGINT"),
        "SQL_C_BIT" => Some("SQLCHAR"),
        _ => None,
    }
}

/// C++ expression reading a scalar from `buf`.
pub fn cpp_scalar_expr(target_type: &str) -> Option<&'static str> {
    match target_type {
        "SQL_C_DOUBLE" => Some("*reinterpret_cast<double*>(buf.data())"),
        "SQL_C_FLOAT" => Some("*reinterpret_cast<float*>(buf.data())"),
        "SQL_C_BIT" => Some("static_cast<SQLCHAR>(buf[0]) != 0"),
        "SQL_C_TINYINT" | "SQL_C_STINYINT" => Some("*reinterpret_cast<SQLSCHAR*>(buf.data())"),
        "SQL_C_UTINYINT" => Some("*reinterpret_cast<SQLCHAR*>(buf.data())"),
        "SQL_C_SHORT" | "SQL_C_SSHORT" => Some("*reinterpret_cast<SQLSMALLINT*>(buf.data())"),
        "SQL_C_USHORT" => Some("*reinterpret_cast<SQLUSMALLINT*>(buf.data())"),
        "SQL_C_LONG" | "SQL_C_SLONG" => Some("*reinterpret_cast<SQLINTEGER*>(buf.data())"),
        "SQL_C_ULONG" => Some("*reinterpret_cast<SQLUINTEGER*>(buf.data())"),
        "SQL_C_SBIGINT" => Some("*reinterpret_cast<SQLBIGINT*>(buf.data())"),
        "SQL_C_UBIGINT" => Some("*reinterpret_cast<SQLUBIGINT*>(buf.data())"),
        _ => None,
    }
}

/// Lines emitted after a successful obscured GetData to record into `captured_values`.
pub fn capture_record_lines(seq: u64, target_type: &str) -> Vec<String> {
    let key = format!("\"{seq}\"");
    let mut lines = vec![
        "{".to_string(),
        "  if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {".to_string(),
        "    if (ind >= 0) {".to_string(),
    ];

    match target_type {
        "SQL_C_DOUBLE" => {
            lines.extend([
                "      double _cv = *reinterpret_cast<double*>(buf.data());".to_string(),
                "      picojson::value inner;".to_string(),
                "      if (std::isnan(_cv)) {".to_string(),
                "        inner = picojson::value(\"NaN\");".to_string(),
                "      } else if (std::isinf(_cv)) {".to_string(),
                "        inner = picojson::value(std::signbit(_cv) ? \"-Infinity\" : \"Infinity\");"
                    .to_string(),
                "      } else {".to_string(),
                "        inner = picojson::value(_cv);".to_string(),
                "      }".to_string(),
                format!(
                    "      picojson::object wrap; wrap[\"{}\"] = inner;",
                    tags::DOUBLE
                ),
                format!("      captured_values[{key}] = picojson::value(wrap);"),
            ]);
        }
        "SQL_C_FLOAT" => {
            lines.extend([
                "      float _cv = *reinterpret_cast<float*>(buf.data());".to_string(),
                "      picojson::value inner;".to_string(),
                "      if (std::isnan(_cv)) {".to_string(),
                "        inner = picojson::value(\"NaN\");".to_string(),
                "      } else if (std::isinf(_cv)) {".to_string(),
                "        inner = picojson::value(std::signbit(_cv) ? \"-Infinity\" : \"Infinity\");"
                    .to_string(),
                "      } else {".to_string(),
                "        inner = picojson::value(static_cast<double>(_cv));".to_string(),
                "      }".to_string(),
                format!(
                    "      picojson::object wrap; wrap[\"{}\"] = inner;",
                    tags::FLOAT
                ),
                format!("      captured_values[{key}] = picojson::value(wrap);"),
            ]);
        }
        "SQL_C_BINARY" | "SQL_C_VARBINARY" => {
            lines.extend([
                "      const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());"
                    .to_string(),
                "      std::string hex;".to_string(),
                "      hex.reserve(n * 2);".to_string(),
                "      for (size_t i = 0; i < n; ++i) {".to_string(),
                "        char tmp[3];".to_string(),
                "        std::snprintf(tmp, sizeof(tmp), \"%02x\", static_cast<unsigned char>(buf[i]));"
                    .to_string(),
                "        hex += tmp;".to_string(),
                "      }".to_string(),
                format!(
                    "      picojson::object wrap; wrap[\"{}\"] = picojson::value(hex);",
                    tags::BYTES
                ),
                format!("      captured_values[{key}] = picojson::value(wrap);"),
            ]);
        }
        t if integer_cpp_type(t).is_some() => {
            let ty = integer_cpp_type(t).unwrap();
            let read = if t == "SQL_C_BIT" {
                "static_cast<long long>(static_cast<SQLCHAR>(buf[0]) != 0)".to_string()
            } else {
                format!("static_cast<long long>(*reinterpret_cast<{ty}*>(buf.data()))")
            };
            lines.extend([
                format!(
                    "      picojson::object wrap; wrap[\"{}\"] = picojson::value(std::to_string({read}));",
                    tags::INT
                ),
                format!("      captured_values[{key}] = picojson::value(wrap);"),
            ]);
        }
        "SQL_C_DATE" | "SQL_C_TYPE_DATE" => {
            lines.extend(date_capture_lines(&key));
        }
        "SQL_C_TIME" | "SQL_C_TYPE_TIME" => {
            lines.extend(time_capture_lines(&key));
        }
        "SQL_C_TIMESTAMP" | "SQL_C_TYPE_TIMESTAMP" => {
            lines.extend(timestamp_capture_lines(&key));
        }
        _ => {
            lines.push(format!("      captured_values[{key}] = picojson::value();"));
        }
    }

    lines.extend([
        "    } else {".to_string(),
        format!("      captured_values[{key}] = picojson::value();"),
        "    }".to_string(),
        "  } else {".to_string(),
        format!("    captured_values[{key}] = picojson::value();"),
        "  }".to_string(),
        "}".to_string(),
    ]);
    lines
}

fn date_capture_lines(key: &str) -> Vec<String> {
    vec![
        "      const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());"
            .to_string(),
        "      picojson::object fields;".to_string(),
        "      fields[\"year\"] = picojson::value(static_cast<double>(_ds->year));".to_string(),
        "      fields[\"month\"] = picojson::value(static_cast<double>(_ds->month));".to_string(),
        "      fields[\"day\"] = picojson::value(static_cast<double>(_ds->day));".to_string(),
        format!(
            "      picojson::object wrap; wrap[\"{}\"] = picojson::value(fields);",
            tags::DATE
        ),
        format!("      captured_values[{key}] = picojson::value(wrap);"),
    ]
}

fn time_capture_lines(key: &str) -> Vec<String> {
    vec![
        "      const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());"
            .to_string(),
        "      picojson::object fields;".to_string(),
        "      fields[\"hour\"] = picojson::value(static_cast<double>(_ts->hour));".to_string(),
        "      fields[\"minute\"] = picojson::value(static_cast<double>(_ts->minute));".to_string(),
        "      fields[\"second\"] = picojson::value(static_cast<double>(_ts->second));".to_string(),
        format!(
            "      picojson::object wrap; wrap[\"{}\"] = picojson::value(fields);",
            tags::TIME
        ),
        format!("      captured_values[{key}] = picojson::value(wrap);"),
    ]
}

fn timestamp_capture_lines(key: &str) -> Vec<String> {
    vec![
        "      const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());"
            .to_string(),
        "      picojson::object fields;".to_string(),
        "      fields[\"year\"] = picojson::value(static_cast<double>(_ts->year));".to_string(),
        "      fields[\"month\"] = picojson::value(static_cast<double>(_ts->month));".to_string(),
        "      fields[\"day\"] = picojson::value(static_cast<double>(_ts->day));".to_string(),
        "      fields[\"hour\"] = picojson::value(static_cast<double>(_ts->hour));".to_string(),
        "      fields[\"minute\"] = picojson::value(static_cast<double>(_ts->minute));".to_string(),
        "      fields[\"second\"] = picojson::value(static_cast<double>(_ts->second));".to_string(),
        "      fields[\"fraction\"] = picojson::value(static_cast<double>(_ts->fraction));"
            .to_string(),
        format!(
            "      picojson::object wrap; wrap[\"{}\"] = picojson::value(fields);",
            tags::TIMESTAMP
        ),
        format!("      captured_values[{key}] = picojson::value(wrap);"),
    ]
}

/// Lines asserting a persisted [`CapturedValue`] against `buf`/`ind`.
pub fn captured_assert_lines(target_type: &str, captured: &CapturedValue) -> Vec<String> {
    match captured {
        CapturedValue::Double(v) => double_assert_lines(target_type, v),
        CapturedValue::Float(v) => float_assert_lines(target_type, v),
        CapturedValue::Bytes(hex) => bytes_assert_lines(hex),
        CapturedValue::Int(s) => int_assert_lines(target_type, s),
        CapturedValue::Date { year, month, day } => vec![
            "const SQL_DATE_STRUCT* _ds = reinterpret_cast<SQL_DATE_STRUCT*>(buf.data());"
                .to_string(),
            format!("CHECK(_ds->year == {year});"),
            format!("CHECK(_ds->month == {month});"),
            format!("CHECK(_ds->day == {day});"),
        ],
        CapturedValue::Time {
            hour,
            minute,
            second,
        } => vec![
            "const SQL_TIME_STRUCT* _ts = reinterpret_cast<SQL_TIME_STRUCT*>(buf.data());"
                .to_string(),
            format!("CHECK(_ts->hour == {hour});"),
            format!("CHECK(_ts->minute == {minute});"),
            format!("CHECK(_ts->second == {second});"),
        ],
        CapturedValue::Timestamp {
            year,
            month,
            day,
            hour,
            minute,
            second,
            fraction,
        } => vec![
            "const SQL_TIMESTAMP_STRUCT* _ts = reinterpret_cast<SQL_TIMESTAMP_STRUCT*>(buf.data());"
                .to_string(),
            format!("CHECK(_ts->year == {year});"),
            format!("CHECK(_ts->month == {month});"),
            format!("CHECK(_ts->day == {day});"),
            format!("CHECK(_ts->hour == {hour});"),
            format!("CHECK(_ts->minute == {minute});"),
            format!("CHECK(_ts->second == {second});"),
            format!("CHECK(_ts->fraction == {fraction});"),
        ],
    }
}

fn double_assert_lines(target_type: &str, val: &DoubleVal) -> Vec<String> {
    let expr = cpp_scalar_expr(target_type).unwrap_or("*reinterpret_cast<double*>(buf.data())");
    match val {
        DoubleVal::Finite(v) => vec![format!("CHECK(({expr}) == {v});")],
        DoubleVal::NonFinite(s) => match s.as_str() {
            "NaN" => vec![format!("CHECK(std::isnan({expr}));")],
            "Infinity" => vec![format!(
                "CHECK((std::isinf({expr}) && !std::signbit({expr})));"
            )],
            "-Infinity" => vec![format!(
                "CHECK((std::isinf({expr}) && std::signbit({expr})));"
            )],
            other => vec![format!("// unsupported non-finite double: {other}")],
        },
    }
}

fn float_assert_lines(target_type: &str, val: &FloatVal) -> Vec<String> {
    let expr = cpp_scalar_expr(target_type).unwrap_or("*reinterpret_cast<float*>(buf.data())");
    match val {
        FloatVal::Finite(v) => vec![format!("CHECK(({expr}) == {v}f);")],
        FloatVal::NonFinite(s) => match s.as_str() {
            "NaN" => vec![format!("CHECK(std::isnan({expr}));")],
            "Infinity" => vec![format!(
                "CHECK((std::isinf({expr}) && !std::signbit({expr})));"
            )],
            "-Infinity" => vec![format!(
                "CHECK((std::isinf({expr}) && std::signbit({expr})));"
            )],
            other => vec![format!("// unsupported non-finite float: {other}")],
        },
    }
}

fn bytes_assert_lines(hex: &str) -> Vec<String> {
    let mut bytes = Vec::new();
    for chunk in hex.as_bytes().chunks(2) {
        if chunk.len() == 2 {
            let s = std::str::from_utf8(chunk).unwrap();
            let b = u8::from_str_radix(s, 16).unwrap();
            bytes.push(b);
        }
    }
    let literal = bytes
        .iter()
        .map(|b| format!("0x{b:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    vec![
        "const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());".to_string(),
        format!("const unsigned char expected[] = {{{literal}}};"),
        "CHECK(n == sizeof(expected));".to_string(),
        "CHECK(std::memcmp(buf.data(), expected, n) == 0);".to_string(),
    ]
}

fn int_assert_lines(target_type: &str, decimal: &str) -> Vec<String> {
    if target_type == "SQL_C_BIT" {
        let expected = if decimal == "1" || decimal == "true" {
            "true".to_string()
        } else {
            "false".to_string()
        };
        return vec![format!(
            "CHECK((static_cast<SQLCHAR>(buf[0]) != 0) == {expected});"
        )];
    }
    let ty = integer_cpp_type(target_type).unwrap_or("SQLINTEGER");
    let expr = format!("*reinterpret_cast<{ty}*>(buf.data())");
    vec![format!("CHECK(({expr}) == static_cast<{ty}>({decimal}));")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obscured_excludes_char_types() {
        assert!(!is_obscured_target("SQL_C_CHAR"));
        assert!(!is_obscured_target("SQL_C_WCHAR"));
        assert!(is_obscured_target("SQL_C_DOUBLE"));
    }

    #[test]
    fn capture_record_includes_seq_key() {
        let lines = capture_record_lines(42, "SQL_C_DOUBLE");
        assert!(lines.iter().any(|l| l.contains("\"42\"")));
        assert!(lines.iter().any(|l| l.contains(tags::DOUBLE)));
    }
}
