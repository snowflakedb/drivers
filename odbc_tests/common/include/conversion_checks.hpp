#ifndef CONVERSION_CHECKS_HPP
#define CONVERSION_CHECKS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "MetaOfSqlCTypes.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"

template <int SQL_C_TYPE>
static typename MetaOfSqlCType<SQL_C_TYPE>::type check_fractional_truncation(const StatementHandleWrapper& stmt,
                                                                             int column) {
  INFO("Checking fractional truncation for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == sizeof(typename MetaOfSqlCType<SQL_C_TYPE>::type));
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "01S07");
  return value;
}

template <int SQL_C_TYPE>
static void check_numeric_out_of_range(const StatementHandleWrapper& stmt, int column) {
  INFO("Checking numeric out of range for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_ERROR);
  // Not checking indicator since it is not guaranteed to be set when ret == SQL_ERROR
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "22003");
}

template <int SQL_C_TYPE>
static typename MetaOfSqlCType<SQL_C_TYPE>::type check_no_truncation(const StatementHandleWrapper& stmt, int column) {
  INFO("Checking no truncation for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(typename MetaOfSqlCType<SQL_C_TYPE>::type));
  return value;
}

template <int SQL_C_TYPE>
static void check_invalid_string(const StatementHandleWrapper& stmt, int column) {
  INFO("Checking invalid string for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "22018");
}

template <int SQL_C_TYPE>
static void check_error(const StatementHandleWrapper& stmt, int column) {
  INFO("Checking error for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 0);
}

// Check for interval trailing field truncation (SQLSTATE 01S07)
template <int SQL_C_TYPE>
static typename MetaOfSqlCType<SQL_C_TYPE>::type check_interval_trailing_truncation(const StatementHandleWrapper& stmt,
                                                                                    int column) {
  INFO("Checking interval trailing field truncation for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == sizeof(typename MetaOfSqlCType<SQL_C_TYPE>::type));
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "01S07");
  return value;
}

// Check for interval leading field precision loss (SQLSTATE 22015)
template <int SQL_C_TYPE>
static void check_interval_precision_lost(const StatementHandleWrapper& stmt, int column) {
  INFO("Checking interval leading field precision lost for column " << column);
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE, &value, &indicator);
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "22015");
}

inline void check_null_via_get_data(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT c_type) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, c_type, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == SQL_NULL_DATA);
}

inline std::string check_char_success(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[8192];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator >= 0);
  return std::string(buffer, indicator);
}

inline std::u16string check_wchar_success(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char16_t buffer[8192];
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator >= 0);
  return std::u16string(buffer, indicator / sizeof(char16_t));
}

// Verifies that a SQLGetData conversion fails with an incompatible-conversion SQLSTATE.
//
// The ODBC spec mandates SQLSTATE 07006 ("Restricted data type attribute violation")
// when the source SQL type cannot be converted to the requested C target type — for
// example, numeric to temporal (DATE/TIME/TIMESTAMP) or numeric to GUID.
//
// On Windows the ODBC Driver Manager may intercept specific unsupported target types
// before the driver is even invoked and return HYC00 ("Optional feature not
// implemented") instead of 07006. Known intercepted types:
//   - SQL_C_GUID (target_type = -11)
// The relaxed check is scoped to _WIN32 builds AND only to these known target types;
// all other target types must return exactly 07006 on every platform.
inline void check_incompatible_conversion(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type,
                                          void* buffer, SQLLEN buffer_size) {
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, target_type, buffer, buffer_size, &indicator);
  auto records = get_diag_rec(stmt);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("target_type=" << target_type << " ret=" << ret << " sqlstate=" << sqlstate);
  REQUIRE(ret == SQL_ERROR);
  REQUIRE(!records.empty());
#ifdef _WIN32
  if (target_type == SQL_C_GUID) {
    CHECK((sqlstate == "07006" || sqlstate == "HYC00"));
  } else {
    CHECK(sqlstate == "07006");
  }
#else
  CHECK(sqlstate == "07006");
#endif
}

// Decodes the first 8 bytes of SQL_NUMERIC_STRUCT.val[] as a little-endian
// unsigned 64-bit integer. Sufficient for values up to 2^64-1.
inline unsigned long long numeric_val_to_ull(const SQL_NUMERIC_STRUCT& n) {
  unsigned long long result = 0;
  for (int i = 7; i >= 0; --i) {
    result = (result << 8) | n.val[i];
  }
  return result;
}

// Asserts that val[start..15] in a SQL_NUMERIC_STRUCT are all zero.
// Use after numeric_val_to_ull to verify the driver did not set stale bytes
// beyond the value's actual byte width.
inline void check_numeric_val_zero_from(const SQL_NUMERIC_STRUCT& numeric, int start) {
  for (int i = start; i < 16; ++i) {
    INFO("val[" << i << "] should be 0");
    CHECK(numeric.val[i] == 0);
  }
}

#endif  // CONVERSION_CHECKS_HPP
