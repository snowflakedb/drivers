use arrow::array::FixedSizeListArray;
use odbc_sys as sql;
use sf_types::VectorCell;

use crate::api::CDataType;
use crate::conversion::error::{ReadArrowError, UnsupportedOdbcTypeSnafu, WriteOdbcError};
use crate::conversion::traits::{Binding, ReadArrowType, SnowflakeType, WriteODBCType};
use crate::conversion::warning::Warnings;

/// Snowflake VECTOR type — serializes to a compact JSON array string.
///
/// The wire value from sf_core is an Arrow `FixedSizeListArray` of `Int32` or
/// `Float32` primitives, decoded by `sf_types::SnowflakeVector`. ODBC exposes
/// it as a JSON string (e.g. `[1,2,3]`) via `SQL_C_CHAR` / `SQL_C_WCHAR` /
/// `SQL_C_BINARY`, matching the ARRAY / VARIANT precedent. The element type is
/// discovered from the Arrow child, so it is not carried on this struct.
pub(crate) struct SnowflakeVector {
    pub column_size: u32,
}

impl SnowflakeType for SnowflakeVector {
    type Representation<'a> = String;
}

impl ReadArrowType<FixedSizeListArray> for SnowflakeVector {
    fn read_arrow_type<'a>(
        &self,
        array: &'a FixedSizeListArray,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        let json = match sf_types::ReadArrowType::read_arrow_type(
            &sf_types::SnowflakeVector,
            array,
            row_idx,
        )? {
            VectorCell::Int32(values) => {
                let mut s = String::with_capacity(values.len() * 4 + 2);
                s.push('[');
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(&v.to_string());
                }
                s.push(']');
                s
            }
            VectorCell::Float32(values) => {
                let mut s = String::with_capacity(values.len() * 8 + 2);
                s.push('[');
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(&format_f32(*v));
                }
                s.push(']');
                s
            }
        };
        Ok(json)
    }
}

/// Format a single f32 value as a JSON-array element string.
///
/// Finite values use Rust's `Display`, whose shortest-round-trip (Grisu3/
/// Dragon4) algorithm preserves subnormal values such as FLOAT32_SMALLEST_NORMAL
/// (~1.1754944e-38) instead of collapsing them to zero.
///
/// Non-finite values are emitted as the bare tokens `NaN`, `Infinity` and
/// `-Infinity`. These are not valid strict JSON, but they are the convention
/// used across the Snowflake driver ecosystem for non-finite floats in
/// semi-structured / VECTOR payloads: old ODBC's picojson serializer, the
/// Snowflake JSON bind parser (see `SnowflakeReal::write_wire`), and JDBC's
/// `List.toString()` all produce exactly these spellings. Note Rust's `Display`
/// would otherwise render infinities as `inf` / `-inf`, which the ecosystem
/// does not accept — hence the explicit mapping here.
fn format_f32(v: f32) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v == f32::INFINITY {
        "Infinity".to_string()
    } else if v == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{v}")
    }
}

impl WriteODBCType for SnowflakeVector {
    fn sql_type(&self) -> sql::SqlDataType {
        sql::SqlDataType::VARCHAR
    }

    fn column_size(&self) -> sql::ULen {
        self.column_size as sql::ULen
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        0
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        match binding.target_type {
            CDataType::Default | CDataType::Char | CDataType::WChar | CDataType::Binary => {}
            _ => {
                return UnsupportedOdbcTypeSnafu {
                    target_type: binding.target_type,
                }
                .fail();
            }
        }
        let s: &str = &snowflake_value;
        match binding.target_type {
            CDataType::Default | CDataType::Char => {
                Ok(binding.write_char_string(s, get_data_offset))
            }
            CDataType::WChar => Ok(binding.write_wchar_string(s, get_data_offset)),
            CDataType::Binary => Ok(binding.write_binary(s.as_bytes(), get_data_offset)),
            _ => unreachable!("all non-string-family types rejected above"),
        }
    }
}
