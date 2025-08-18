/* Copied from https://github.com/pacman82/odbc-sys */

/* Orignal license: */
/*
MIT License

Copyright (c) 2017

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

pub const C_TYPES_EXTENDED: i16 = 0x04000;

#[repr(i16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    // SQL_C_TINYINT = -6,
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
