#ifndef CONVERSION_MATRIX_COMMON_HPP
#define CONVERSION_MATRIX_COMMON_HPP

#ifdef ENABLE_PROGRESS_REPORT
#define SKIP_UNLESS_PROGRESS_REPORT() ((void)0)
#else
#define SKIP_UNLESS_PROGRESS_REPORT() SKIP("Skipping progress report disabled")
#endif

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <fstream>
#include <mutex>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"

// ---------------------------------------------------------------------------
// C type registry
// ---------------------------------------------------------------------------

struct CTypeInfo {
  SQLSMALLINT c_type;
  const char* name;
  size_t buffer_size;
};

// clang-format off
static const CTypeInfo ALL_C_TYPES[] = {
  {SQL_C_CHAR,                        "SQL_C_CHAR",                        256},
  {SQL_C_WCHAR,                       "SQL_C_WCHAR",                       512},
  {SQL_C_BIT,                         "SQL_C_BIT",                         sizeof(SQLCHAR)},
  {SQL_C_TINYINT,                     "SQL_C_TINYINT",                     sizeof(SQLSCHAR)},
  {SQL_C_STINYINT,                    "SQL_C_STINYINT",                    sizeof(SQLSCHAR)},
  {SQL_C_UTINYINT,                    "SQL_C_UTINYINT",                    sizeof(SQLCHAR)},
  {SQL_C_SHORT,                       "SQL_C_SHORT",                       sizeof(SQLSMALLINT)},
  {SQL_C_SSHORT,                      "SQL_C_SSHORT",                      sizeof(SQLSMALLINT)},
  {SQL_C_USHORT,                      "SQL_C_USHORT",                      sizeof(SQLUSMALLINT)},
  {SQL_C_LONG,                        "SQL_C_LONG",                        sizeof(SQLINTEGER)},
  {SQL_C_SLONG,                       "SQL_C_SLONG",                       sizeof(SQLINTEGER)},
  {SQL_C_ULONG,                       "SQL_C_ULONG",                       sizeof(SQLUINTEGER)},
  {SQL_C_SBIGINT,                     "SQL_C_SBIGINT",                     sizeof(SQLBIGINT)},
  {SQL_C_UBIGINT,                     "SQL_C_UBIGINT",                     sizeof(SQLUBIGINT)},
  {SQL_C_FLOAT,                       "SQL_C_FLOAT",                       sizeof(SQLREAL)},
  {SQL_C_DOUBLE,                      "SQL_C_DOUBLE",                      sizeof(SQLDOUBLE)},
  {SQL_C_NUMERIC,                     "SQL_C_NUMERIC",                     sizeof(SQL_NUMERIC_STRUCT)},
  {SQL_C_BINARY,                      "SQL_C_BINARY",                      256},
  {SQL_C_TYPE_DATE,                   "SQL_C_TYPE_DATE",                   sizeof(SQL_DATE_STRUCT)},
  {SQL_C_TYPE_TIME,                   "SQL_C_TYPE_TIME",                   sizeof(SQL_TIME_STRUCT)},
  {SQL_C_TYPE_TIMESTAMP,              "SQL_C_TYPE_TIMESTAMP",              sizeof(SQL_TIMESTAMP_STRUCT)},
  {SQL_C_INTERVAL_YEAR,               "SQL_C_INTERVAL_YEAR",               sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_MONTH,              "SQL_C_INTERVAL_MONTH",              sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_DAY,                "SQL_C_INTERVAL_DAY",                sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_HOUR,               "SQL_C_INTERVAL_HOUR",               sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_MINUTE,             "SQL_C_INTERVAL_MINUTE",             sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_SECOND,             "SQL_C_INTERVAL_SECOND",             sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_YEAR_TO_MONTH,      "SQL_C_INTERVAL_YEAR_TO_MONTH",      sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_DAY_TO_HOUR,        "SQL_C_INTERVAL_DAY_TO_HOUR",        sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_DAY_TO_MINUTE,      "SQL_C_INTERVAL_DAY_TO_MINUTE",      sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_DAY_TO_SECOND,      "SQL_C_INTERVAL_DAY_TO_SECOND",      sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_HOUR_TO_MINUTE,     "SQL_C_INTERVAL_HOUR_TO_MINUTE",     sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_HOUR_TO_SECOND,     "SQL_C_INTERVAL_HOUR_TO_SECOND",     sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_INTERVAL_MINUTE_TO_SECOND,   "SQL_C_INTERVAL_MINUTE_TO_SECOND",   sizeof(SQL_INTERVAL_STRUCT)},
  {SQL_C_GUID,                        "SQL_C_GUID",                        sizeof(SQLGUID)},
};
// clang-format on

static constexpr size_t NUM_C_TYPES = sizeof(ALL_C_TYPES) / sizeof(ALL_C_TYPES[0]);

// ---------------------------------------------------------------------------
// SQL type registry (for SQLBindParameter target types)
// ---------------------------------------------------------------------------

struct SqlTypeInfo {
  SQLSMALLINT sql_type;
  const char* name;
};

// ---------------------------------------------------------------------------
// Result reporting
// ---------------------------------------------------------------------------

static const char* sqlreturn_name(SQLRETURN ret) {
  switch (ret) {
    case SQL_SUCCESS:
      return "SQL_SUCCESS";
    case SQL_SUCCESS_WITH_INFO:
      return "SQL_SUCCESS_WITH_INFO";
    case SQL_ERROR:
      return "SQL_ERROR";
    case SQL_NO_DATA:
      return "SQL_NO_DATA";
    case SQL_NEED_DATA:
      return "SQL_NEED_DATA";
    case SQL_INVALID_HANDLE:
      return "SQL_INVALID_HANDLE";
    default:
      return "UNKNOWN";
  }
}

class ResultWriter {
 public:
  explicit ResultWriter(const std::string& filename) : filename_(filename) {
    std::ofstream f(filename_, std::ios::trunc);
    f << "direction,sql_type,c_type,result,sqlstate,message\n";
  }

  void write(const char* direction, const char* sql_type, const char* c_type, SQLRETURN ret,
             const std::string& sqlstate = "", const std::string& message = "") {
    std::lock_guard<std::mutex> lock(mutex_);
    std::ofstream f(filename_, std::ios::app);
    f << direction << "," << sql_type << "," << c_type << "," << sqlreturn_name(ret) << "," << sqlstate << ","
      << escape_csv(message) << "\n";
  }

 private:
  static std::string escape_csv(const std::string& s) {
    std::string flat = s;
    for (char& c : flat) {
      if (c == '\n' || c == '\r') c = ' ';
    }
    if (flat.find_first_of(",\"") == std::string::npos) return flat;
    std::string out = "\"";
    for (char c : flat) {
      if (c == '"')
        out += "\"\"";
      else
        out += c;
    }
    out += "\"";
    return out;
  }

  std::string filename_;
  std::mutex mutex_;
};

static std::string get_report_path(const char* name) {
  const char* dir = std::getenv("CM_REPORT_DIR");
  std::string base = dir ? dir : ".";
  return base + "/conversion_matrix_" + name + ".csv";
}

// ---------------------------------------------------------------------------
// SQLGetData helper -- never fails, writes result to CSV
// ---------------------------------------------------------------------------

static void try_getdata(Connection& conn, const char* query, const char* sql_type_name, const CTypeInfo& ct,
                        ResultWriter& report) {
  auto stmt = conn.execute_fetch(query);
  char buffer[1024] = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, ct.c_type, buffer, static_cast<SQLLEN>(ct.buffer_size), &indicator);

  std::string sqlstate;
  std::string message;
  if (ret == SQL_ERROR || ret == SQL_SUCCESS_WITH_INFO) {
    auto records = get_diag_rec(stmt);
    if (!records.empty()) {
      sqlstate = records[0].sqlState;
      message = records[0].messageText;
    }
  }
  report.write("getdata", sql_type_name, ct.name, ret, sqlstate, message);
}

// ---------------------------------------------------------------------------
// Sample value buffer for SQLBindParameter
// ---------------------------------------------------------------------------

struct SampleValue {
  char raw[1024] = {};
  SQLLEN indicator = 0;
  SQLLEN buffer_length = 0;
};

static SampleValue make_sample_value(SQLSMALLINT c_type) {
  SampleValue sv;
  std::memset(sv.raw, 0, sizeof(sv.raw));

  switch (c_type) {
    case SQL_C_CHAR: {
      const char* text = "42";
      std::strncpy(sv.raw, text, sizeof(sv.raw) - 1);
      sv.indicator = SQL_NTS;
      sv.buffer_length = static_cast<SQLLEN>(std::strlen(text) + 1);
      break;
    }
    case SQL_C_WCHAR: {
      const char16_t text[] = u"42";
      std::memcpy(sv.raw, text, sizeof(text));
      sv.indicator = SQL_NTS;
      sv.buffer_length = static_cast<SQLLEN>(sizeof(text));
      break;
    }
    case SQL_C_BIT: {
      SQLCHAR val = 1;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_TINYINT:
    case SQL_C_STINYINT: {
      SQLSCHAR val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_UTINYINT: {
      SQLCHAR val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_SHORT:
    case SQL_C_SSHORT: {
      SQLSMALLINT val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_USHORT: {
      SQLUSMALLINT val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_LONG:
    case SQL_C_SLONG: {
      SQLINTEGER val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_ULONG: {
      SQLUINTEGER val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_SBIGINT: {
      SQLBIGINT val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_UBIGINT: {
      SQLUBIGINT val = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_FLOAT: {
      SQLREAL val = 42.0f;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_DOUBLE: {
      SQLDOUBLE val = 42.0;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_NUMERIC: {
      SQL_NUMERIC_STRUCT val = {};
      val.precision = 10;
      val.scale = 0;
      val.sign = 1;
      val.val[0] = 42;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_BINARY: {
      SQLCHAR bytes[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F};
      std::memcpy(sv.raw, bytes, sizeof(bytes));
      sv.indicator = sizeof(bytes);
      sv.buffer_length = sizeof(bytes);
      break;
    }
    case SQL_C_TYPE_DATE: {
      SQL_DATE_STRUCT val = {};
      val.year = 2024;
      val.month = 1;
      val.day = 15;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_TYPE_TIME: {
      SQL_TIME_STRUCT val = {};
      val.hour = 12;
      val.minute = 30;
      val.second = 45;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_TYPE_TIMESTAMP: {
      SQL_TIMESTAMP_STRUCT val = {};
      val.year = 2024;
      val.month = 1;
      val.day = 15;
      val.hour = 12;
      val.minute = 30;
      val.second = 45;
      val.fraction = 0;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_INTERVAL_YEAR:
    case SQL_C_INTERVAL_MONTH:
    case SQL_C_INTERVAL_DAY:
    case SQL_C_INTERVAL_HOUR:
    case SQL_C_INTERVAL_MINUTE:
    case SQL_C_INTERVAL_SECOND:
    case SQL_C_INTERVAL_YEAR_TO_MONTH:
    case SQL_C_INTERVAL_DAY_TO_HOUR:
    case SQL_C_INTERVAL_DAY_TO_MINUTE:
    case SQL_C_INTERVAL_DAY_TO_SECOND:
    case SQL_C_INTERVAL_HOUR_TO_MINUTE:
    case SQL_C_INTERVAL_HOUR_TO_SECOND:
    case SQL_C_INTERVAL_MINUTE_TO_SECOND: {
      SQL_INTERVAL_STRUCT val = {};
      val.interval_type = SQL_IS_YEAR;
      val.interval_sign = SQL_FALSE;
      val.intval.year_month.year = 1;
      val.intval.year_month.month = 0;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    case SQL_C_GUID: {
      SQLGUID val = {};
      val.Data1 = 0x12345678;
      val.Data2 = 0x1234;
      val.Data3 = 0x1234;
      std::memcpy(sv.raw, &val, sizeof(val));
      sv.indicator = sizeof(val);
      sv.buffer_length = sizeof(val);
      break;
    }
    default: {
      const char* fallback = "42";
      std::strncpy(sv.raw, fallback, sizeof(sv.raw) - 1);
      sv.indicator = SQL_NTS;
      sv.buffer_length = static_cast<SQLLEN>(std::strlen(fallback) + 1);
      break;
    }
  }
  return sv;
}

// ---------------------------------------------------------------------------
// SQLBindParameter helper -- never fails, writes result to CSV
// ---------------------------------------------------------------------------

static void try_bindparam(Connection& conn, const char* insert_sql, const CTypeInfo& ct, const SqlTypeInfo& st,
                          ResultWriter& report) {
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar(insert_sql), SQL_NTS);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    std::string sqlstate;
    auto records = get_diag_rec(stmt);
    if (!records.empty()) sqlstate = records[0].sqlState;
    report.write("bindparam", st.name, ct.name, ret, sqlstate, "SQLPrepare failed");
    return;
  }

  auto sv = make_sample_value(ct.c_type);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, ct.c_type, st.sql_type, 0, 0, sv.raw, sv.buffer_length,
                         &sv.indicator);
  if (ret == SQL_ERROR) {
    std::string sqlstate;
    std::string message;
    auto records = get_diag_rec(stmt);
    if (!records.empty()) {
      sqlstate = records[0].sqlState;
      message = records[0].messageText;
    }
    report.write("bindparam", st.name, ct.name, ret, sqlstate, "SQLBindParameter: " + message);
    return;
  }

  ret = SQLExecute(stmt.getHandle());
  std::string sqlstate;
  std::string message;
  if (ret == SQL_ERROR || ret == SQL_SUCCESS_WITH_INFO) {
    auto records = get_diag_rec(stmt);
    if (!records.empty()) {
      sqlstate = records[0].sqlState;
      message = records[0].messageText;
    }
  }
  report.write("bindparam", st.name, ct.name, ret, sqlstate, message);
}

#endif  // CONVERSION_MATRIX_COMMON_HPP
