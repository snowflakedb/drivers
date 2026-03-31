// C data types — copied from https://github.com/pacman82/odbc-sys
//
// MIT License
//
// Copyright (c) 2017
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::api::OdbcError;
use crate::api::error::InvalidApplicationBufferTypeSnafu;
use odbc_sys as sql;

pub const C_TYPES_EXTENDED: i16 = 0x04000;

pub const SQL_NULL_DATA: sql::Len = -1;

#[repr(i16)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum CDataType {
    /// SQL_ARD_TYPE
    Ard = -99,

    /// SQL_APD_TYPE
    Apd = -100,

    UTinyInt = -28,
    UBigInt = -27,
    STinyInt = -26,
    SBigInt = -25,

    ULong = -18,
    UShort = -17,
    SLong = -16,
    SShort = -15,

    // #[cfg(feature = "odbc_version_3_50")]
    Guid = -11,

    WChar = -8,

    Bit = -7,
    // deprecated
    TinyInt = -6,
    Binary = -2,
    /// `SQLCHAR` - CHAR, VARCHAR, DECIMAL, NUMERIC
    Char = 1,
    Numeric = 2,

    // deprecated
    Long = 4,
    Short = 5,
    Float = 7,
    Double = 8,

    // Used in Odbc2.x Odbc3.x uses TypeDate instead.
    Date = 9,
    // Used in Odbc2.x Odbc3.x uses TypeTime instead.
    Time = 10,
    // Used in Odbc2.x Odbc3.x uses TypeTimeTimestamp instead.
    TimeStamp = 11,

    /// SQL_TYPE_DATE
    TypeDate = 91,
    /// SQL_TYPE_TIME
    TypeTime = 92,
    /// SQL_TYPE_TIMESTAMP
    TypeTimestamp = 93,
    // #[cfg(feature = "odbc_version_4")]
    TypeTimeWithTimezone = 94,
    // #[cfg(feature = "odbc_version_4")]
    TypeTimestampWithTimezone = 95,

    #[default]
    Default = 99,

    IntervalYear = 101,
    IntervalMonth = 102,
    IntervalDay = 103,
    IntervalHour = 104,
    IntervalMinute = 105,
    IntervalSecond = 106,
    IntervalYearToMonth = 107,
    IntervalDayToHour = 108,
    IntervalDayToMinute = 109,
    IntervalDayToSecond = 110,
    IntervalHourToMinute = 111,
    IntervalHourToSecond = 112,
    IntervalMinuteToSecond = 113,

    SsTime2 = C_TYPES_EXTENDED,
    SsTimestampOffset = C_TYPES_EXTENDED + 1,
}

impl CDataType {
    /// Returns the byte size of fixed-length C data types, or `None` for
    /// variable-length types (CHAR, WCHAR, BINARY).
    pub fn fixed_size(&self) -> Option<usize> {
        match self {
            CDataType::Bit | CDataType::TinyInt | CDataType::STinyInt | CDataType::UTinyInt => {
                Some(1)
            }
            CDataType::Short | CDataType::SShort | CDataType::UShort => Some(2),
            CDataType::Long | CDataType::SLong | CDataType::ULong | CDataType::Float => Some(4),
            CDataType::Double | CDataType::SBigInt | CDataType::UBigInt => Some(8),
            CDataType::Numeric => Some(std::mem::size_of::<sql::Numeric>()),
            CDataType::TypeDate | CDataType::Date => Some(std::mem::size_of::<sql::Date>()),
            CDataType::TypeTimestamp | CDataType::TimeStamp => {
                Some(std::mem::size_of::<sql::Timestamp>())
            }
            CDataType::TypeTime | CDataType::Time => Some(std::mem::size_of::<sql::Time>()),
            CDataType::Guid => Some(16),
            CDataType::IntervalYear
            | CDataType::IntervalMonth
            | CDataType::IntervalDay
            | CDataType::IntervalHour
            | CDataType::IntervalMinute
            | CDataType::IntervalSecond
            | CDataType::IntervalYearToMonth
            | CDataType::IntervalDayToHour
            | CDataType::IntervalDayToMinute
            | CDataType::IntervalDayToSecond
            | CDataType::IntervalHourToMinute
            | CDataType::IntervalHourToSecond
            | CDataType::IntervalMinuteToSecond => Some(std::mem::size_of::<sql::IntervalStruct>()),
            _ => None,
        }
    }
}

impl TryFrom<i16> for CDataType {
    type Error = OdbcError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            -100 => Ok(CDataType::Apd),
            -99 => Ok(CDataType::Ard),
            -28 => Ok(CDataType::UTinyInt),
            -27 => Ok(CDataType::UBigInt),
            -26 => Ok(CDataType::STinyInt),
            -25 => Ok(CDataType::SBigInt),
            -18 => Ok(CDataType::ULong),
            -17 => Ok(CDataType::UShort),
            -16 => Ok(CDataType::SLong),
            -15 => Ok(CDataType::SShort),
            -11 => Ok(CDataType::Guid),
            -8 => Ok(CDataType::WChar),
            -7 => Ok(CDataType::Bit),
            -6 => Ok(CDataType::TinyInt),
            -2 => Ok(CDataType::Binary),
            1 => Ok(CDataType::Char),
            2 => Ok(CDataType::Numeric),
            4 => Ok(CDataType::Long),
            5 => Ok(CDataType::Short),
            7 => Ok(CDataType::Float),
            8 => Ok(CDataType::Double),
            9 => Ok(CDataType::Date),
            10 => Ok(CDataType::Time),
            11 => Ok(CDataType::TimeStamp),
            91 => Ok(CDataType::TypeDate),
            92 => Ok(CDataType::TypeTime),
            93 => Ok(CDataType::TypeTimestamp),
            94 => Ok(CDataType::TypeTimeWithTimezone),
            95 => Ok(CDataType::TypeTimestampWithTimezone),
            99 => Ok(CDataType::Default),
            101 => Ok(CDataType::IntervalYear),
            102 => Ok(CDataType::IntervalMonth),
            103 => Ok(CDataType::IntervalDay),
            104 => Ok(CDataType::IntervalHour),
            105 => Ok(CDataType::IntervalMinute),
            106 => Ok(CDataType::IntervalSecond),
            107 => Ok(CDataType::IntervalYearToMonth),
            108 => Ok(CDataType::IntervalDayToHour),
            109 => Ok(CDataType::IntervalDayToMinute),
            110 => Ok(CDataType::IntervalDayToSecond),
            111 => Ok(CDataType::IntervalHourToMinute),
            112 => Ok(CDataType::IntervalHourToSecond),
            113 => Ok(CDataType::IntervalMinuteToSecond),
            x if x == C_TYPES_EXTENDED => Ok(CDataType::SsTime2),
            x if x == C_TYPES_EXTENDED + 1 => Ok(CDataType::SsTimestampOffset),
            _ => InvalidApplicationBufferTypeSnafu.fail(),
        }
    }
}

#[allow(dead_code)]
pub type UBigInt = u64;
#[allow(dead_code)]
pub type SBigInt = i64;
#[allow(dead_code)]
pub type Real = f32;
#[allow(dead_code)]
pub type Double = f64;
