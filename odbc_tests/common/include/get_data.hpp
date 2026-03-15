#ifndef GET_DATA_HPP
#define GET_DATA_HPP

#include <sql.h>
#include <sqlext.h>

#include <optional>
#include <string>

#include "HandleWrapper.hpp"
#include "MetaOfSqlCTypes.hpp"
#include "odbc_matchers.hpp"

template <int SQL_C_TYPE>
inline std::optional<typename MetaOfSqlCType<SQL_C_TYPE>::type> get_data_optional(const StatementHandleWrapper& stmt,
                                                                                  SQLUSMALLINT col) {
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_TYPE, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  if (indicator == SQL_NULL_DATA) {
    return std::nullopt;
  }
  return value;
}

// Template specialization for SQL_C_CHAR to return std::string
template <>
inline std::optional<std::string> get_data_optional<SQL_C_CHAR>(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[8192];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  if (indicator == SQL_NULL_DATA) {
    return std::nullopt;
  }
  return {std::string(buffer, indicator)};
}

template <>
inline std::optional<std::u16string> get_data_optional<SQL_C_WCHAR>(const StatementHandleWrapper& stmt,
                                                                    SQLUSMALLINT col) {
  char16_t buffer[8192];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  if (indicator == SQL_NULL_DATA) {
    return std::nullopt;
  }
  return {std::u16string(buffer, indicator / sizeof(char16_t))};
}

template <int SQL_C_TYPE>
inline typename MetaOfSqlCType<SQL_C_TYPE>::type get_data(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  auto optional_value = get_data_optional<SQL_C_TYPE>(stmt, col);
  REQUIRE(optional_value.has_value());
  return optional_value.value();
}

template <typename T>
inline SQLRETURN get_data_raw(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type, T* value,
                              SQLLEN* indicator) {
  return SQLGetData(stmt.getHandle(), col, target_type, value, sizeof(*value), indicator);
}

#endif  // GET_DATA_HPP
