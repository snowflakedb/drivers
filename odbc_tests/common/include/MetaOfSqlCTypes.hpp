
#ifndef ODBC_TESTS_SQLCTYPES_HPP
#define ODBC_TESTS_SQLCTYPES_HPP

#include <sql.h>
#include <sqlext.h>

#include <string>

template <int SQL_C_TYPE>
class MetaOfSqlCType {};

template <>
class MetaOfSqlCType<SQL_C_LONG> {
 public:
  using type = SQLINTEGER;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_LONG;
  std::string name() { return "SQL_C_LONG"; }
};

template <>
class MetaOfSqlCType<SQL_C_SLONG> {
 public:
  using type = SQLINTEGER;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_SLONG;
  std::string name() { return "SQL_C_SLONG"; }
};

template <>
class MetaOfSqlCType<SQL_C_ULONG> {
 public:
  using type = SQLUINTEGER;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_ULONG;
  std::string name() { return "SQL_C_ULONG"; }
};

template <>
class MetaOfSqlCType<SQL_C_SHORT> {
 public:
  using type = SQLSMALLINT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_SHORT;
  std::string name() { return "SQL_C_SHORT"; }
};

template <>
class MetaOfSqlCType<SQL_C_SSHORT> {
 public:
  using type = SQLSMALLINT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_SSHORT;
  std::string name() { return "SQL_C_SSHORT"; }
};

template <>
class MetaOfSqlCType<SQL_C_USHORT> {
 public:
  using type = SQLUSMALLINT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_USHORT;
  std::string name() { return "SQL_C_USHORT"; }
};

template <>
class MetaOfSqlCType<SQL_C_TINYINT> {
 public:
  using type = SQLSCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_TINYINT;
  std::string name() { return "SQL_C_TINYINT"; }
};

template <>
class MetaOfSqlCType<SQL_C_STINYINT> {
 public:
  using type = SQLSCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_STINYINT;
  std::string name() { return "SQL_C_STINYINT"; }
};

template <>
class MetaOfSqlCType<SQL_C_UTINYINT> {
 public:
  using type = SQLCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_UTINYINT;
  std::string name() { return "SQL_C_UTINYINT"; }
};

template <>
class MetaOfSqlCType<SQL_C_FLOAT> {
 public:
  using type = SQLREAL;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_FLOAT;
  std::string name() { return "SQL_C_FLOAT"; }
};

template <>
class MetaOfSqlCType<SQL_C_DOUBLE> {
 public:
  using type = SQLDOUBLE;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_DOUBLE;
  std::string name() { return "SQL_C_DOUBLE"; }
};

template <>
class MetaOfSqlCType<SQL_C_CHAR> {
 public:
  using type = std::string;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_CHAR;
  std::string name() { return "SQL_C_CHAR"; }
};

template <>
class MetaOfSqlCType<SQL_C_WCHAR> {
 public:
  using type = SQLWCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_WCHAR;
  std::string name() { return "SQL_C_WCHAR"; }
};

template <>
class MetaOfSqlCType<SQL_C_BINARY> {
 public:
  using type = SQLCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_BINARY;
  std::string name() { return "SQL_C_BINARY"; }
};

template <>
class MetaOfSqlCType<SQL_C_BIT> {
 public:
  using type = SQLCHAR;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_BIT;
  std::string name() { return "SQL_C_BIT"; }
};

template <>
class MetaOfSqlCType<SQL_C_SBIGINT> {
 public:
  using type = SQLBIGINT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_SBIGINT;
  std::string name() { return "SQL_C_SBIGINT"; }
};

template <>
class MetaOfSqlCType<SQL_C_UBIGINT> {
 public:
  using type = SQLUBIGINT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_UBIGINT;
  std::string name() { return "SQL_C_UBIGINT"; }
};

template <>
class MetaOfSqlCType<SQL_C_NUMERIC> {
 public:
  using type = SQL_NUMERIC_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_NUMERIC;
  std::string name() { return "SQL_C_NUMERIC"; }
};

template <>
class MetaOfSqlCType<SQL_C_TYPE_DATE> {
 public:
  using type = SQL_DATE_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_TYPE_DATE;
  std::string name() { return "SQL_C_TYPE_DATE"; }
};

template <>
class MetaOfSqlCType<SQL_C_TYPE_TIME> {
 public:
  using type = SQL_TIME_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_TYPE_TIME;
  std::string name() { return "SQL_C_TYPE_TIME"; }
};

template <>
class MetaOfSqlCType<SQL_C_TYPE_TIMESTAMP> {
 public:
  using type = SQL_TIMESTAMP_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_TYPE_TIMESTAMP;
  std::string name() { return "SQL_C_TIMESTAMP"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_YEAR> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_YEAR;
  std::string name() { return "SQL_C_INTERVAL_YEAR"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_MONTH> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_MONTH;
  std::string name() { return "SQL_C_INTERVAL_MONTH"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_DAY> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_DAY;
  std::string name() { return "SQL_C_INTERVAL_DAY"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_HOUR> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_HOUR;
  std::string name() { return "SQL_C_INTERVAL_HOUR"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_MINUTE> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_MINUTE;
  std::string name() { return "SQL_C_INTERVAL_MINUTE"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_SECOND> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_SECOND;
  std::string name() { return "SQL_C_INTERVAL_SECOND"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_YEAR_TO_MONTH> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_YEAR_TO_MONTH;
  std::string name() { return "SQL_C_INTERVAL_YEAR_TO_MONTH"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_DAY_TO_HOUR> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_DAY_TO_HOUR;
  std::string name() { return "SQL_C_INTERVAL_DAY_TO_HOUR"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_DAY_TO_MINUTE> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_DAY_TO_MINUTE;
  std::string name() { return "SQL_C_INTERVAL_DAY_TO_MINUTE"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_DAY_TO_SECOND> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_DAY_TO_SECOND;
  std::string name() { return "SQL_C_INTERVAL_DAY_TO_SECOND"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_HOUR_TO_MINUTE> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_HOUR_TO_MINUTE;
  std::string name() { return "SQL_C_INTERVAL_HOUR_TO_MINUTE"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_HOUR_TO_SECOND> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_HOUR_TO_SECOND;
  std::string name() { return "SQL_C_INTERVAL_HOUR_TO_SECOND"; }
};

template <>
class MetaOfSqlCType<SQL_C_INTERVAL_MINUTE_TO_SECOND> {
 public:
  using type = SQL_INTERVAL_STRUCT;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_INTERVAL_MINUTE_TO_SECOND;
  std::string name() { return "SQL_C_INTERVAL_MINUTE_TO_SECOND"; }
};

template <>
class MetaOfSqlCType<SQL_C_GUID> {
 public:
  using type = SQLGUID;
  static constexpr SQLSMALLINT sql_c_type = SQL_C_GUID;
  std::string name() { return "SQL_C_GUID"; }
};

#endif  // ODBC_TESTS_SQLCTYPES_HPP
