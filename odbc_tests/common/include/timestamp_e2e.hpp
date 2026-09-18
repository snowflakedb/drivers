#ifndef TIMESTAMP_E2E_HPP
#define TIMESTAMP_E2E_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"

// SQLDescribeCol and SQL_DESC_TYPE_NAME both collapse NTZ/LTZ/TZ to
// SQL_TYPE_TIMESTAMP / TYPE_TIMESTAMP. SYSTEM$TYPEOF reports the Snowflake type
// that carries or omits timezone. TYPEOF is VARIANT-only.
inline void require_timezone_info(const std::string& typeof_name) {
  INFO(typeof_name);
  REQUIRE((typeof_name.find("TIMESTAMP_LTZ") != std::string::npos ||
           typeof_name.find("TIMESTAMP_TZ") != std::string::npos));
}

inline void require_no_timezone_info(const std::string& typeof_name) {
  INFO(typeof_name);
  REQUIRE(typeof_name.find("TIMESTAMP_NTZ") != std::string::npos);
}

inline SQL_TIMESTAMP_STRUCT make_ts(SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day, SQLUSMALLINT hour,
                                    SQLUSMALLINT minute, SQLUSMALLINT second, SQLUINTEGER fraction = 0) {
  return SQL_TIMESTAMP_STRUCT{year, month, day, hour, minute, second, fraction};
}

inline void require_ts(const SQL_TIMESTAMP_STRUCT& got, SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day,
                       SQLUSMALLINT hour, SQLUSMALLINT minute, SQLUSMALLINT second, SQLUINTEGER fraction = 0) {
  REQUIRE(got.year == year);
  REQUIRE(got.month == month);
  REQUIRE(got.day == day);
  REQUIRE(got.hour == hour);
  REQUIRE(got.minute == minute);
  REQUIRE(got.second == second);
  REQUIRE(got.fraction == fraction);
}

inline void require_sql_timestamp(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLSMALLINT data_type = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, nullptr, nullptr, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type == SQL_TYPE_TIMESTAMP);
}

inline void require_sequential_from_2024(const StatementHandleWrapper& stmt, int expected_rows) {
  int row_count = 0;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    INFO("row=" << row_count);
    require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1), 2024, 1, 1, static_cast<SQLUSMALLINT>(row_count / 3600),
               static_cast<SQLUSMALLINT>((row_count % 3600) / 60), static_cast<SQLUSMALLINT>(row_count % 60));
    row_count++;
  }
  REQUIRE(row_count == expected_rows);
}

inline void bind_ts(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQL_TIMESTAMP_STRUCT& value,
                    SQLLEN& indicator) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP,
                                   29, 9, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
}

#endif  // TIMESTAMP_E2E_HPP
