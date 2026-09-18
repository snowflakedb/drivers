#ifndef GEO_TEST_HELPERS_HPP
#define GEO_TEST_HELPERS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

#include "HandleWrapper.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

inline std::string fetch_char(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  return get_data<SQL_C_CHAR>(stmt, col);
}

inline void require_sql_type(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT expected) {
  SQLSMALLINT data_type = 0;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, nullptr, nullptr, nullptr);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(data_type == expected);
}

inline void require_contains(const std::string& haystack, const char* needle) {
  REQUIRE(haystack.find(needle) != std::string::npos);
}

inline void require_geojson_shape(const std::string& geo, const char* shape) {
  require_contains(geo, "\"type\"");
  require_contains(geo, shape);
}

inline std::vector<SQLCHAR> fetch_binary(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  std::vector<SQLCHAR> buffer(4096, 0xFF);
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer.data(), static_cast<SQLLEN>(buffer.size()), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator > 0);
  buffer.resize(static_cast<size_t>(indicator));
  return buffer;
}

inline void bind_varchar(const StatementHandleWrapper& stmt, SQLUSMALLINT param, char* value, SQLLEN& indicator) {
  const auto len = static_cast<SQLLEN>(std::strlen(value));
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, value,
                                   len + 1, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

#endif  // GEO_TEST_HELPERS_HPP
