#ifndef GET_DATA_HPP
#define GET_DATA_HPP

#include <sql.h>
#include <sqlext.h>

#include <string>

#include "HandleWrapper.hpp"
#include "MetaOfSqlCTypes.hpp"

template <int SQL_C_TYPE>
inline typename MetaOfSqlCType<SQL_C_TYPE>::type get_data(const StatementHandleWrapper& stmt,
                                                          SQLUSMALLINT col) {
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_TYPE, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  return value;
}

// Template specialization for SQL_C_CHAR to return std::string
template <>
inline std::string get_data<SQL_C_CHAR>(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[1000];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  return std::string(buffer, indicator);
}

#endif  // GET_DATA_HPP
