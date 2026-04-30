#ifndef SNOWFLAKE_ODBC_CONSTANTS_HPP
#define SNOWFLAKE_ODBC_CONSTANTS_HPP

#include <sql.h>
#include <sqltypes.h>

// Snowflake-specific vendor SQL type codes for TIMESTAMP variants. Defined in
// the legacy 3.16.0 driver's `Source/sf_odbc.h` and reported by
// `SQLDescribeCol` and `SQLColAttribute` so applications can distinguish
// `TIMESTAMP_LTZ` / `_TZ` / `_NTZ` columns that would otherwise all surface as
// the standard `SQL_TYPE_TIMESTAMP` (93). Vendor codes >= 2000 are explicitly
// permitted by the MS ODBC specification ("Driver-specific data types").
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_LTZ = 2000;
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_TZ = 2001;
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_NTZ = 2002;

// Column size (display width) reported for `TIMESTAMP_TZ` columns. Large enough
// for `yyyy-mm-dd HH:MM:SS.fffffffff +/-HH:MM`.
constexpr SQLULEN SQL_SF_TIMESTAMP_TZ_COLUMN_SIZE = 35;

#endif  // SNOWFLAKE_ODBC_CONSTANTS_HPP
