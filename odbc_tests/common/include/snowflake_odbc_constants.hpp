#ifndef SNOWFLAKE_ODBC_CONSTANTS_HPP
#define SNOWFLAKE_ODBC_CONSTANTS_HPP

#include <sql.h>
#include <sqltypes.h>

// Snowflake-specific vendor SQL type codes for TIMESTAMP variants. Defined in
// the legacy 3.16.0 driver's `Source/sf_odbc.h`. These are accepted as the
// `ParameterType` argument to `SQLBindParameter` so applications can
// explicitly request `TIMESTAMP_LTZ` / `_TZ` / `_NTZ` round-trip behavior
// that the standard `SQL_TYPE_TIMESTAMP` (93) cannot distinguish.
//
// They are NOT returned from `SQLDescribeCol` or
// `SQLColAttribute(SQL_DESC_CONCISE_TYPE)`; those report the spec-mandated
// `SQL_TYPE_TIMESTAMP` (93) and applications use `SQL_DESC_TYPE_NAME` to tell
// the three subtypes apart.
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_LTZ = 2000;
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_TZ = 2001;
constexpr SQLSMALLINT SQL_SF_TIMESTAMP_NTZ = 2002;

#endif  // SNOWFLAKE_ODBC_CONSTANTS_HPP
