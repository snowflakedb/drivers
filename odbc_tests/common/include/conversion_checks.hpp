#ifndef CONVERSION_CHECKS_HPP
#define CONVERSION_CHECKS_HPP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

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

// Snowflake may serialize null values as the bare token "undefined" in
// semi-structured types (ARRAY, OBJECT, VARIANT).  This is not valid JSON,
// so we replace it with "null" before parsing.  The token only appears in
// value positions (never inside quoted strings), so a simple find-replace
// is safe for the known Snowflake output format.
inline std::string sanitize_json(const std::string& text) {
  std::string result = text;
  size_t pos = 0;
  while ((pos = result.find("undefined", pos)) != std::string::npos) {
    result.replace(pos, 9, "null");
    pos += 4;
  }
  return result;
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
// Platform / driver exceptions:
//   - Windows DM: may return HYC00 for SQL_C_GUID before the driver is invoked.
//   - Old (reference) driver: may return 22018 ("Invalid character value for cast
//     specification") or 22003 ("Numeric value out of range") for semi-structured
//     types (ARRAY/OBJECT/VARIANT) because it attempts the conversion rather than
//     rejecting the target type upfront. Pass is_semi_structured=true for those callers.
inline void check_incompatible_conversion(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type,
                                          void* buffer, SQLLEN buffer_size, bool is_semi_structured = false) {
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, target_type, buffer, buffer_size, &indicator);
  auto records = get_diag_rec(stmt);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("target_type=" << target_type << " ret=" << ret << " sqlstate=" << sqlstate);
  REQUIRE(ret == SQL_ERROR);
  REQUIRE(!records.empty());
#ifdef SNOWFLAKE_OLD_DRIVER
  if (is_semi_structured) {
    CHECK((sqlstate == "07006" || sqlstate == "22018" || sqlstate == "22003"));
  } else {
    CHECK(sqlstate == "07006");
  }
#elif defined(_WIN32)
  if (target_type == SQL_C_GUID) {
    CHECK((sqlstate == "07006" || sqlstate == "HYC00"));
  } else {
    CHECK(sqlstate == "07006");
  }
#else
  CHECK(sqlstate == "07006");
#endif
}

inline std::string get_data_default_as_string(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[1000];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator >= 0);
  REQUIRE(indicator < static_cast<SQLLEN>(sizeof(buffer)));
  return std::string(buffer, indicator);
}

inline SQL_NUMERIC_STRUCT get_binary_as_numeric(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == sizeof(SQL_NUMERIC_STRUCT));
  SQL_NUMERIC_STRUCT result;
  std::memcpy(&result, buffer, sizeof(SQL_NUMERIC_STRUCT));
  return result;
}

inline SQL_NUMERIC_STRUCT get_binary_as_numeric_with_truncation(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(indicator == sizeof(SQL_NUMERIC_STRUCT));
  auto records = get_diag_rec(stmt);
  CHECK(records.size() == 1);
  CHECK(records[0].sqlState == "01S07");
  SQL_NUMERIC_STRUCT result;
  std::memcpy(&result, buffer, sizeof(SQL_NUMERIC_STRUCT));
  return result;
}

template <int SQL_C_TYPE>
void check_integer_columns(const StatementHandleWrapper& stmt, const std::vector<int>& exact_cols,
                           const std::vector<int>& truncated_cols, typename MetaOfSqlCType<SQL_C_TYPE>::type expected) {
  for (int col : exact_cols) {
    INFO("Column " << col << " with " << MetaOfSqlCType<SQL_C_TYPE>().name() << " (exact)");
    CHECK(check_no_truncation<SQL_C_TYPE>(stmt, col) == expected);
  }
  for (int col : truncated_cols) {
    INFO("Column " << col << " with " << MetaOfSqlCType<SQL_C_TYPE>().name() << " (truncated)");
    CHECK(check_fractional_truncation<SQL_C_TYPE>(stmt, col) == expected);
  }
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

inline void check_incompatible_bindparam(const HandleWrapper& stmt_handle, SQLSMALLINT c_type, SQLSMALLINT sql_type,
                                         void* value, SQLLEN buffer_len, SQLLEN* ind) {
  SQLRETURN ret =
      SQLBindParameter(stmt_handle.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, 0, 0, value, buffer_len, ind);
  if (ret == SQL_ERROR) {
    auto records = get_diag_rec(stmt_handle);
    INFO("c_type=" << c_type << " sql_type=" << sql_type << " rejected at SQLBindParameter");
    REQUIRE(!records.empty());
#ifdef _WIN32
    if (c_type == SQL_C_GUID) {
      CHECK((records[0].sqlState == "07006" || records[0].sqlState == "HYC00"));
    } else {
      CHECK(records[0].sqlState == "07006");
    }
#else
    CHECK(records[0].sqlState == "07006");
#endif
    return;
  }
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecute(stmt_handle.getHandle());
  auto records = get_diag_rec(stmt_handle);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("c_type=" << c_type << " sql_type=" << sql_type << " ret=" << ret << " sqlstate=" << sqlstate);
  REQUIRE(ret == SQL_ERROR);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "07006");
}

inline void set_numeric_magnitude(SQL_NUMERIC_STRUCT& ns, uint64_t magnitude) {
  std::memset(ns.val, 0, sizeof(ns.val));
  std::memcpy(ns.val, &magnitude, sizeof(magnitude));
}

inline void set_numeric_magnitude_128(SQL_NUMERIC_STRUCT& ns, uint64_t low, uint64_t high) {
  std::memset(ns.val, 0, sizeof(ns.val));
  std::memcpy(ns.val, &low, sizeof(low));
  std::memcpy(ns.val + sizeof(low), &high, sizeof(high));
}

#endif  // CONVERSION_CHECKS_HPP
