#ifndef ODBC_RETURN_CODE_HPP
#define ODBC_RETURN_CODE_HPP

#include <sql.h>

#include <string>

inline std::string return_code_to_string(SQLRETURN ret) {
  switch (ret) {
    case SQL_SUCCESS:
      return "SQL_SUCCESS";
    case SQL_SUCCESS_WITH_INFO:
      return "SQL_SUCCESS_WITH_INFO";
    case SQL_ERROR:
      return "SQL_ERROR";
    case SQL_INVALID_HANDLE:
      return "SQL_INVALID_HANDLE";
    case SQL_NO_DATA:
      return "SQL_NO_DATA";
    case SQL_NEED_DATA:
      return "SQL_NEED_DATA";
    case SQL_STILL_EXECUTING:
      return "SQL_STILL_EXECUTING";
    default:
      return "UNKNOWN_RETURN_CODE(" + std::to_string(ret) + ")";
  }
}

#endif  // ODBC_RETURN_CODE_HPP
