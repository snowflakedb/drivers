//! ODBC result-side INTERVAL readers.
//!
//! The Arrow decode — one signed integer per cell — is shared through
//! `sf_types::SnowflakeIntervalYearMonth` (total months) and
//! `SnowflakeIntervalDayTime` (total nanoseconds). This module adds the ODBC
//! output half: the canonical ANSI literal (`[-]Y-MM`, `[-]D HH:MM:SS[.f]`)
//! that `SQL_C_CHAR`/`SQL_C_WCHAR` receive, which the bind-side
//! [`varchar_to_interval`] parser also re-reads for same-family
//! `SQL_C_INTERVAL_*` targets. The bind-side structs of the same Snowflake
//! types live in `interval.rs` and implement the parameter path
//! (`ReadODBC`/`WriteWire`), so the result readers are separate types here.
//!
//! `column_size`/`display_size` follow the ODBC interval-length rules over the
//! ANSI literal: Snowflake result metadata carries no interval leading
//! precision, so the driver's default of two leading digits
//! ([`INTERVAL_LEADING_PRECISION`]) applies.
//!
//! A scalar numeric target (`SQL_C_SBIGINT`, `SQL_C_NUMERIC`, and the other
//! integer C types, plus `SQL_C_BINARY`) receives the interval's total months
//! (YEAR TO MONTH) or total whole seconds (DAY TO SECOND). ODBC Appendix D does
//! not define an interval-to-numeric conversion for compound qualifiers, so the
//! day-time value drops its sub-second component and reports 01S07 truncation
//! when nanoseconds are lost.

use arrow::array::PrimitiveArray;
use arrow::datatypes::ArrowPrimitiveType;
use odbc_sys as sql;

use crate::api::CDataType;
use crate::conversion::error::{ReadArrowError, WriteOdbcError};
use crate::conversion::interval_str::varchar_to_interval;
use crate::conversion::numeric_helpers::write_integer_as_numeric;
use crate::conversion::traits::Binding;
use crate::conversion::warning::Warnings;
use crate::conversion::{ReadArrowType, SnowflakeType, WriteODBCType};
use sf_output_format::{format_day_time, format_year_month};

/// SQL_INTERVAL_YEAR_TO_MONTH. odbc_sys 0.25.1 exposes no interval
/// `SqlDataType` constants, so the concise type code is named here.
const SQL_INTERVAL_YEAR_TO_MONTH: sql::SqlDataType = sql::SqlDataType(107);
/// SQL_INTERVAL_DAY_TO_SECOND.
const SQL_INTERVAL_DAY_TO_SECOND: sql::SqlDataType = sql::SqlDataType(110);

/// Interval leading-field precision assumed for length metadata. Snowflake
/// result columns carry no leading precision, and the driver defaults
/// `SQL_DESC_DATETIME_INTERVAL_PRECISION` to 2.
const INTERVAL_LEADING_PRECISION: sql::ULen = 2;

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Snowflake INTERVAL YEAR TO MONTH result column, decoded as a total month
/// count and rendered as the ANSI `[-]Y-MM` literal.
pub(crate) struct IntervalYearMonthReader;

impl SnowflakeType for IntervalYearMonthReader {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for IntervalYearMonthReader
where
    T::Native: Into<i128>,
{
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<T>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        Ok(sf_types::ReadArrowType::read_arrow_type(
            &sf_types::SnowflakeIntervalYearMonth,
            array,
            row_idx,
        )?)
    }
}

impl WriteODBCType for IntervalYearMonthReader {
    fn sql_type(&self) -> sql::SqlDataType {
        SQL_INTERVAL_YEAR_TO_MONTH
    }

    fn column_size(&self) -> sql::ULen {
        INTERVAL_LEADING_PRECISION + 3
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
        let literal = format_year_month(snowflake_value);
        match binding.target_type {
            CDataType::Default | CDataType::Char => {
                Ok(binding.write_char_string(&literal, get_data_offset))
            }
            CDataType::WChar => Ok(binding.write_wchar_string(&literal, get_data_offset)),
            CDataType::IntervalYear | CDataType::IntervalMonth | CDataType::IntervalYearToMonth => {
                varchar_to_interval(&literal, binding.target_type, binding)
            }
            target_type => write_integer_as_numeric(target_type, snowflake_value, false, binding),
        }
    }
}

/// Snowflake INTERVAL DAY TO SECOND result column, decoded as a total
/// nanosecond count and rendered as the ANSI `[-]D HH:MM:SS[.f]` literal.
pub(crate) struct IntervalDayTimeReader {
    pub scale: u32,
}

impl SnowflakeType for IntervalDayTimeReader {
    type Representation<'a> = i128;
}

impl<T: ArrowPrimitiveType> ReadArrowType<PrimitiveArray<T>> for IntervalDayTimeReader
where
    T::Native: Into<i128>,
{
    fn read_arrow_type<'a>(
        &self,
        array: &'a PrimitiveArray<T>,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError> {
        Ok(sf_types::ReadArrowType::read_arrow_type(
            &sf_types::SnowflakeIntervalDayTime,
            array,
            row_idx,
        )?)
    }
}

impl WriteODBCType for IntervalDayTimeReader {
    fn sql_type(&self) -> sql::SqlDataType {
        SQL_INTERVAL_DAY_TO_SECOND
    }

    fn column_size(&self) -> sql::ULen {
        let base = INTERVAL_LEADING_PRECISION + 9;
        if self.scale > 0 {
            base + 1 + self.scale as sql::ULen
        } else {
            base
        }
    }

    fn decimal_digits(&self) -> sql::SmallInt {
        self.scale as sql::SmallInt
    }

    fn write_odbc_type(
        &self,
        snowflake_value: Self::Representation<'_>,
        binding: &Binding,
        get_data_offset: &mut Option<usize>,
    ) -> Result<Warnings, WriteOdbcError> {
        let literal = format_day_time(snowflake_value, self.scale);
        match binding.target_type {
            CDataType::Default | CDataType::Char => {
                Ok(binding.write_char_string(&literal, get_data_offset))
            }
            CDataType::WChar => Ok(binding.write_wchar_string(&literal, get_data_offset)),
            CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute
            | CDataType::IntervalSecond
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => {
                varchar_to_interval(&literal, binding.target_type, binding)
            }
            target_type => {
                let int_value = snowflake_value / NANOS_PER_SECOND as i128;
                let has_fractional = snowflake_value % NANOS_PER_SECOND as i128 != 0;
                write_integer_as_numeric(target_type, int_value, has_fractional, binding)
            }
        }
    }
}
