#include <sql.h>
#include <sqlext.h>

#include <cstring>
#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "ScopedTable.hpp"
#include "SessionParameterOverride.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "query_helpers.hpp"
#include "timestamp_e2e.hpp"

static std::string bulk_insert_id_name(Connection& conn, const std::string& table, int count, int id_offset,
                                       const std::string& name_prefix) {
  constexpr int NAME_BUF = 32;
  std::vector<SQLBIGINT> ids(count);
  std::vector<char> names(static_cast<size_t>(count) * NAME_BUF, '\0');
  std::vector<SQLLEN> id_inds(count, 0), name_inds(count);
  std::vector<SQLUSMALLINT> param_status(count, 0);
  SQLULEN params_processed = 0;

  for (int i = 0; i < count; i++) {
    ids[i] = static_cast<SQLBIGINT>(id_offset + i);
    std::string name = name_prefix + std::to_string(i);
    std::strncpy(&names[static_cast<size_t>(i) * NAME_BUF], name.c_str(), NAME_BUF - 1);
    name_inds[i] = static_cast<SQLLEN>(name.size());
  }

  auto stmt = conn.createStatement();
  SQLRETURN ret;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE,
                       reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(count)), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status.data(), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ids.data(),
                         sizeof(SQLBIGINT), id_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 255, 0, names.data(), NAME_BUF,
                         name_inds.data());
  REQUIRE_ODBC(ret, stmt);

  const std::string sql = "INSERT INTO " + table + " VALUES (?, ?)";
  ret = SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == static_cast<SQLULEN>(count));
  for (int i = 0; i < count; i++) {
    REQUIRE(param_status[i] != SQL_PARAM_ERROR);
  }

  return get_last_query_id(stmt);
}

static void check_id_name_row(StatementHandleWrapper& stmt, int64_t expected_id, const std::string& expected_name) {
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == expected_id);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 2) == expected_name);
}

// Bulk-inserts `count` rows of the JDBC stage-bind type matrix via column-wise
// ODBC array binding. Every row shares the same payload except id = 42 + row.
// 13200 rows × 10 cols = 132000 cells, above the default 65280 threshold.
static std::string bulk_insert_types(Connection& conn, const std::string& table, int count) {
  constexpr int NUM_BUF = 48;
  constexpr int TXT_BUF = 48;
  constexpr int BIN_LEN = 8;
  constexpr const char* kNumber = "12345678901234567890123456789.123456789";
  constexpr const char* kText = "stage-bind-text";
  constexpr SQLCHAR binary[BIN_LEN] = {0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef};
  const std::string text(kText);
  const SQL_DATE_STRUCT date{2024, 1, 15};
  const SQL_TIME_STRUCT time{13, 14, 15};
  const SQL_TIMESTAMP_STRUCT ltz = make_ts(2024, 1, 15, 10, 30, 0, 123456789);
  const SQL_TIMESTAMP_STRUCT ntz = make_ts(2023, 6, 20, 14, 22, 33, 987654321);

  std::vector<SQLBIGINT> ids(count);
  std::vector<char> numbers(static_cast<size_t>(count) * NUM_BUF, '\0');
  std::vector<double> fs(count, -12345.625);
  std::vector<SQLCHAR> flags(count, 1);
  std::vector<char> txts(static_cast<size_t>(count) * TXT_BUF, '\0');
  std::vector<SQLCHAR> bins(static_cast<size_t>(count) * BIN_LEN);
  std::vector<SQL_DATE_STRUCT> dates(count, date);
  std::vector<SQL_TIME_STRUCT> times(count, time);
  std::vector<SQL_TIMESTAMP_STRUCT> ltzs(count, ltz);
  std::vector<SQL_TIMESTAMP_STRUCT> ntzs(count, ntz);
  std::vector<SQLLEN> id_inds(count, 0), n_inds(count), f_inds(count, 0), flag_inds(count, 0), txt_inds(count),
      bin_inds(count), date_inds(count, sizeof(SQL_DATE_STRUCT)), time_inds(count, sizeof(SQL_TIME_STRUCT)),
      ltz_inds(count, sizeof(SQL_TIMESTAMP_STRUCT)), ntz_inds(count, sizeof(SQL_TIMESTAMP_STRUCT));
  std::vector<SQLUSMALLINT> param_status(count, 0);
  SQLULEN params_processed = 0;

  for (int i = 0; i < count; i++) {
    ids[i] = 42 + i;
    std::strncpy(&numbers[static_cast<size_t>(i) * NUM_BUF], kNumber, NUM_BUF - 1);
    n_inds[i] = static_cast<SQLLEN>(std::strlen(kNumber));
    std::strncpy(&txts[static_cast<size_t>(i) * TXT_BUF], text.c_str(), TXT_BUF - 1);
    txt_inds[i] = static_cast<SQLLEN>(text.size());
    std::memcpy(&bins[static_cast<size_t>(i) * BIN_LEN], binary, BIN_LEN);
    bin_inds[i] = BIN_LEN;
  }

  auto stmt = conn.createStatement();
  SQLRETURN ret;
  // ODBC 3.x turns SQL_ATTR_ENABLE_AUTO_IPD on by default, so SQLPrepare overwrites the parameter
  // SQL types with the ones the server describes. Preparing first leaves the types passed to
  // SQLBindParameter in effect; otherwise BINARY is described as text and reaches the stage CSV as
  // raw bytes rather than hex.
  const std::string sql = "INSERT INTO " + table + " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
  ret = SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE,
                       reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(count)), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status.data(), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ids.data(),
                         sizeof(SQLBIGINT), id_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_NUMERIC, 38, 9, numbers.data(), NUM_BUF,
                         n_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, fs.data(),
                         sizeof(double), f_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 4, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, flags.data(), sizeof(SQLCHAR),
                         flag_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 5, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 255, 0, txts.data(), TXT_BUF,
                         txt_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 6, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, 50, 0, bins.data(), BIN_LEN,
                         bin_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 7, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 10, 0, dates.data(),
                         sizeof(SQL_DATE_STRUCT), date_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 8, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIME, 8, 0, times.data(),
                         sizeof(SQL_TIME_STRUCT), time_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 9, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9,
                         ltzs.data(), sizeof(SQL_TIMESTAMP_STRUCT), ltz_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 10, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9,
                         ntzs.data(), sizeof(SQL_TIMESTAMP_STRUCT), ntz_inds.data());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == static_cast<SQLULEN>(count));
  for (int i = 0; i < count; i++) {
    REQUIRE(param_status[i] != SQL_PARAM_ERROR);
  }

  return get_last_query_id(stmt);
}

static void check_type_matrix_row(StatementHandleWrapper& stmt, int row) {
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 42 + row);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "12345678901234567890123456789.123456789");
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == Catch::Approx(-12345.625));
  CHECK(get_data<SQL_C_BIT>(stmt, 4) == 1);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 5) == "stage-bind-text");

  SQLCHAR bin[8];
  std::memset(bin, 0xff, sizeof(bin));
  SQLLEN bin_ind = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 6, SQL_C_BINARY, bin, sizeof(bin), &bin_ind);
  REQUIRE_ODBC(ret, stmt);
  CHECK(bin_ind == 8);
  const SQLCHAR expected_bin[8] = {0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef};
  CHECK(std::memcmp(bin, expected_bin, sizeof(bin)) == 0);

  const auto date = get_data<SQL_C_TYPE_DATE>(stmt, 7);
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
  const auto time = get_data<SQL_C_TYPE_TIME>(stmt, 8);
  CHECK(time.hour == 13);
  CHECK(time.minute == 14);
  CHECK(time.second == 15);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 9), 2024, 1, 15, 10, 30, 0, 123456789);
  require_ts(get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 10), 2023, 6, 20, 14, 22, 33, 987654321);
}

static constexpr const char* kCsvHazardCycle[] = {
    "val,0", "say\"1\"",  "a\nb", "C:\\dir\\3", "",     nullptr, "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e",
    "\"",    ",",         "\n",   "\r\n",       "\"\"", "null",  "\\\n",
    "\",",   "\\\",\\\"",
};
static constexpr int kCsvHazardCycleLen = static_cast<int>(sizeof(kCsvHazardCycle) / sizeof(kCsvHazardCycle[0]));
static constexpr int kJapaneseHazardIndex = 6;

static std::string bulk_insert_hazard_strings(Connection& conn, const std::string& table, int count) {
  constexpr int TXT_BUF = 128;
  std::vector<SQLBIGINT> ids(count);
  std::vector<char> txts(static_cast<size_t>(count) * TXT_BUF, '\0');
  std::vector<SQLLEN> id_inds(count, 0), txt_inds(count);
  std::vector<SQLUSMALLINT> param_status(count, 0);
  SQLULEN params_processed = 0;

  for (int i = 0; i < count; i++) {
    ids[i] = static_cast<SQLBIGINT>(i);
    const char* hazard = kCsvHazardCycle[i % kCsvHazardCycleLen];
    if (hazard == nullptr) {
      txt_inds[i] = SQL_NULL_DATA;
    } else {
      std::strncpy(&txts[static_cast<size_t>(i) * TXT_BUF], hazard, TXT_BUF - 1);
      txt_inds[i] = static_cast<SQLLEN>(std::strlen(hazard));
    }
  }

  auto stmt = conn.createStatement();
  SQLRETURN ret;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE,
                       reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(count)), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status.data(), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ids.data(),
                         sizeof(SQLBIGINT), id_inds.data());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 1024, 0, txts.data(), TXT_BUF,
                         txt_inds.data());
  REQUIRE_ODBC(ret, stmt);

  const std::string sql = "INSERT INTO " + table + " VALUES (?, ?)";
  ret = SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == static_cast<SQLULEN>(count));
  for (int i = 0; i < count; i++) {
    REQUIRE(param_status[i] != SQL_PARAM_ERROR);
  }

  return get_last_query_id(stmt);
}

static void insert_all_null_row(Connection& conn, const std::string& table) {
  SQLINTEGER integer_value = 0;
  double double_value = 0;
  float float_value = 0;
  char varchar_value = '\0';
  SQLBIGINT number_value = 0;
  SQLLEN null_indicator = SQL_NULL_DATA;
  SQLUSMALLINT param_status = 0xFFFF;
  SQLULEN params_processed = 0;

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(1)),
                       0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, &param_status, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &integer_value,
                         sizeof(integer_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &double_value,
                         sizeof(double_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &float_value,
                         sizeof(float_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 4, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 255, 0, &varchar_value,
                         sizeof(varchar_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 5, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_NUMERIC, 38, 0, &number_value,
                         sizeof(number_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 6, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &integer_value,
                         sizeof(integer_value), &null_indicator);
  REQUIRE_ODBC(ret, stmt);

  const std::string sql = "INSERT INTO " + table + " VALUES (?, ?, ?, ?, ?, ?)";
  ret = SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(params_processed == 1);
  CHECK(param_status == SQL_PARAM_SUCCESS);
}

static void check_all_null_row(Connection& conn, const std::string& table) {
  auto verify = conn.execute_fetch("SELECT id, colA, colB, colC, colD, colE FROM " + table);
  CHECK(get_data_optional<SQL_C_SLONG>(verify, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_DOUBLE>(verify, 2) == std::nullopt);
  CHECK(get_data_optional<SQL_C_FLOAT>(verify, 3) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 4) == std::nullopt);
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 5) == std::nullopt);
  CHECK(get_data_optional<SQL_C_SLONG>(verify, 6) == std::nullopt);
  CHECK(SQLFetch(verify.getHandle()) == SQL_NO_DATA);
}

// =============================================================================
// Tests
// =============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should stage-bind at the default threshold and reuse SYSTEM$BIND across consecutive bulk inserts",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  ScopedTable table(conn, "lb_threshold_reuse", "id BIGINT, name VARCHAR");

  // When 33000 rows generated as [[i, "first-" + i] for i in 0..33000] are inserted using multirow binding
  auto before1 = list_system_bind_file_count(conn);  // nullopt: @SYSTEM$BIND not yet created
  std::string qid1 = bulk_insert_id_name(conn, table.name(), 33000, 0, "first-");

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after1 = list_system_bind_file_count(conn);
  INFO("First INSERT query_id: " << qid1);
  CHECK(after1 > before1);

  // When 33000 rows generated as [[33000 + i, "second-" + i] for i in 0..33000] are inserted using multirow binding
  std::string qid2 = bulk_insert_id_name(conn, table.name(), 33000, 33000, "second-");

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after2 = list_system_bind_file_count(conn);
  INFO("Second INSERT query_id: " << qid2);
  CHECK(after2 > after1);

  // And Query "SELECT id, name FROM {table} ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, name FROM " + table.name() +
                                   " WHERE id IN (0, 1, 32999, 33000, 65999) ORDER BY id");

  // Then Result should contain the same values as the bound parameters from both bulk inserts
  check_id_name_row(verify, 0, "first-0");

  SQLRETURN ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 1, "first-1");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 32999, "first-32999");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 33000, "second-0");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 65999, "second-32999");

  ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should round-trip all bindable types via stage binding",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in
  auto session_stmt = conn.createStatement();
  SessionParameterOverride timezone_override(session_stmt.getHandle(), "TIMEZONE", "'UTC'");
  SessionParameterOverride timestamp_mapping_override(session_stmt.getHandle(), "CLIENT_TIMESTAMP_TYPE_MAPPING",
                                                      "'TIMESTAMP_NTZ'");

  // And A temporary table with the driver-specific stage-binding type matrix exists
  ScopedTable table(conn, "lb_types",
                    "id NUMBER, n NUMBER(38, 9), f FLOAT, flag BOOLEAN, txt VARCHAR, b BINARY, "
                    "d DATE, t TIME, ts_ltz TIMESTAMP_LTZ, ts_ntz TIMESTAMP_NTZ");

  // When 13200 rows of driver-specific stage-binding values are inserted using multirow binding
  constexpr int row_count = 13200;
  auto before = list_system_bind_file_count(conn);
  std::string qid = bulk_insert_types(conn, table.name(), row_count);

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  CHECK(after > before);

  // And All type-matrix columns are selected from the table in row order
  auto verify =
      conn.execute_fetch("SELECT id, n, f, flag, txt, b, d, t, ts_ltz, ts_ntz FROM " + table.name() + " ORDER BY id");

  // Then Result should contain the same values as the bound parameters
  for (int row = 0; row < row_count; row++) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(verify.getHandle());
      REQUIRE_ODBC(ret, verify);
    }
    INFO("row=" << row);
    check_type_matrix_row(verify, row);
  }
  SQLRETURN ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should preserve CSV escaping hazards via stage binding",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, txt VARCHAR) exists
  ScopedTable table(conn, "lb_hazards", "id BIGINT, txt VARCHAR");

  // When 33000 rows are inserted using multirow binding with values cycling every 16 rows through [[0, "val,0"], [1,
  // "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"], [7, "\""], [8, ","], [9, "\n"],
  // [10,
  // "\r\n"], [11, "\"\""], [12, "null"], [13, "\\\n"], [14, "\","], [15, "\\\",\\\""]]
  auto before = list_system_bind_file_count(conn);
  std::string qid = bulk_insert_hazard_strings(conn, table.name(), 33000);

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  CHECK(after > before);

  // And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 15 ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, txt FROM " + table.name() + " WHERE id BETWEEN 0 AND 15 ORDER BY id");

  // Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL],
  // [6, "日本語"], [7, "\""], [8, ","], [9, "\n"], [10, "\r\n"], [11, "\"\""], [12, "null"], [13, "\\\n"], [14, "\","],
  // [15, "\\\",\\\""]]
  for (int id = 0; id < kCsvHazardCycleLen; id++) {
    if (id > 0) {
      SQLRETURN ret = SQLFetch(verify.getHandle());
      REQUIRE_ODBC(ret, verify);
    }
    INFO("id=" << id);
    CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == id);
    if (id == kJapaneseHazardIndex) {
      SQLCHAR buffer[128];
      memset(buffer, 0xFF, sizeof(buffer));
      SQLLEN indicator = 0;
      SQLRETURN ret = SQLGetData(verify.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
      REQUIRE_ODBC(ret, verify);
      WINDOWS_ONLY {
        // Win-1252 double-encoding: 9 UTF-8 bytes → 19 bytes (see string_conversion_to_c_binary).
        CHECK(indicator == 19);
        CHECK(buffer[0] == 0xC3);
        CHECK(buffer[1] == 0xA6);
        CHECK(buffer[2] == 0xE2);
        CHECK(buffer[3] == 0x80);
        CHECK(buffer[4] == 0x94);
      }
      UNIX_ONLY {
        // Raw UTF-8: [E6 97 A5 E6 9C AC E8 AA 9E]
        CHECK(indicator == 9);
        CHECK(buffer[0] == 0xE6);
        CHECK(buffer[1] == 0x97);
        CHECK(buffer[2] == 0xA5);
        CHECK(buffer[3] == 0xE6);
        CHECK(buffer[4] == 0x9C);
        CHECK(buffer[5] == 0xAC);
        CHECK(buffer[6] == 0xE8);
        CHECK(buffer[7] == 0xAA);
        CHECK(buffer[8] == 0x9E);
      }
    } else if (kCsvHazardCycle[id] == nullptr) {
      CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == std::nullopt);
    } else {
      CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == std::string(kCsvHazardCycle[id]));
    }
  }

  SQLRETURN ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should not stage-bind scalar or non-INSERT queries even when threshold is crossed",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 1
  auto session_stmt = conn.createStatement();
  SessionParameterOverride threshold_override(session_stmt.getHandle(), "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD", "1");
  auto before_count = list_system_bind_file_count(conn);

  // When "SELECT ? AS val" is executed with bound integer value 42
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN ind = 0;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param,
                                   sizeof(SQLINTEGER), &ind);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR select_sql[] = "SELECT ? AS val";
  ret = SQLPrepare(stmt.getHandle(), select_sql, SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bind file on SYSTEM$BIND from the last execute should not contain the bound parameter values
  auto after_count = list_system_bind_file_count(conn);
  INFO("SELECT query_id: " << get_last_query_id(stmt));
  CHECK(after_count == before_count);

  // And the result should equal 42
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_SLONG>(stmt, 1) == 42);
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should use inline JSON when row count is below CLIENT_STAGE_ARRAY_BINDING_THRESHOLD",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  ScopedTable table(conn, "lb_below_threshold", "id BIGINT, name VARCHAR");

  // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 100
  auto session_stmt = conn.createStatement();
  SessionParameterOverride threshold_override(session_stmt.getHandle(), "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD", "100");
  auto before = list_system_bind_file_count(conn);

  // When 10 rows generated as [[i, "json-" + i] for i in 0..10] are inserted using multirow binding
  std::string qid = bulk_insert_id_name(conn, table.name(), 10, 0, "json-");

  // Then no new bind file should have been uploaded to SYSTEM$BIND
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  CHECK(after == before);

  // And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, name FROM " + table.name() + " WHERE id IN (0, 9) ORDER BY id");

  // Then Result should contain rows [[0, "json-0"], [9, "json-9"]]
  check_id_name_row(verify, 0, "json-0");
  SQLRETURN ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 9, "json-9");
  ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should use stage binding at exact threshold boundary", "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  ScopedTable table(conn, "lb_at_threshold", "id BIGINT, name VARCHAR");

  // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 20
  auto session_stmt = conn.createStatement();
  SessionParameterOverride threshold_override(session_stmt.getHandle(), "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD", "20");
  auto before = list_system_bind_file_count(conn);

  // When 10 rows generated as [[i, "stage-" + i] for i in 0..10] are inserted using multirow binding
  std::string qid = bulk_insert_id_name(conn, table.name(), 10, 0, "stage-");

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  // At the exact boundary (20 bound cells == threshold) the new driver uses >= and takes the
  // stage-binding path (bind file uploaded); the old driver uses > and stays on inline JSON (BD#78).
  NEW_DRIVER_ONLY("BD#78") { CHECK(after > before); }
  OLD_DRIVER_ONLY("BD#78") { CHECK(after == before); }

  // And Query "SELECT id, name FROM {table} WHERE id IN (0, 9) ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, name FROM " + table.name() + " WHERE id IN (0, 9) ORDER BY id");

  // Then Result should contain rows [[0, "stage-0"], [9, "stage-9"]]
  check_id_name_row(verify, 0, "stage-0");
  SQLRETURN ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  check_id_name_row(verify, 9, "stage-9");
  ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should keep an all-NULL row on the inline JSON path when stage binding is disabled",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER)
  // exists
  ScopedTable table(conn, "lb_all_null_inline",
                    "id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER");

  // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 0
  auto session_stmt = conn.createStatement();
  SessionParameterOverride threshold_override(session_stmt.getHandle(), "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD", "0");
  auto before = list_system_bind_file_count(conn);

  // When a batch of one row with every column set to SQL NULL is inserted using multirow binding
  insert_all_null_row(conn, table.name());

  // Then no new bind file should have been uploaded to SYSTEM$BIND
  auto after = list_system_bind_file_count(conn);
  CHECK(after == before);

  // And every column of the round-tripped row reads back as SQL NULL
  check_all_null_row(conn, table.name());
}

TEST_CASE_METHOD(ConnSchemaFixture, "should stage-bind an all-NULL row when the bound cell count meets the threshold",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER)
  // exists
  ScopedTable table(conn, "lb_all_null_stage",
                    "id INTEGER, colA DOUBLE, colB FLOAT, colC VARCHAR, colD NUMBER, colE INTEGER");

  // And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter is set to 6
  auto session_stmt = conn.createStatement();
  SessionParameterOverride threshold_override(session_stmt.getHandle(), "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD", "6");
  auto before = list_system_bind_file_count(conn);

  // When a batch of one row with every column set to SQL NULL is inserted using multirow binding
  insert_all_null_row(conn, table.name());

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  NEW_DRIVER_ONLY("BD#78") { CHECK(after > before); }
  OLD_DRIVER_ONLY("BD#78") { CHECK(after == before); }

  // And every column of the round-tripped row reads back as SQL NULL
  check_all_null_row(conn, table.name());
}

// SNOW-3235553: SQL_ATTR_PARAM_OPERATION_PTR — parameter sets marked
// SQL_PARAM_IGNORE are skipped during array execution and reported as
// SQL_PARAM_UNUSED, while still counting toward SQL_ATTR_PARAMS_PROCESSED_PTR.
TEST_CASE_METHOD(ConnSchemaFixture, "should skip SQL_PARAM_IGNORE sets during array execution",
                 "[query][large_bindings][param_operation_ptr]") {
  // Given Snowflake client is logged in
  // And A temporary table with an id column exists
  ScopedTable table(conn, "lb_param_ignore", "id BIGINT");

  constexpr SQLULEN num_rows = 5;
  SQLBIGINT ids[num_rows] = {10, 20, 30, 40, 50};
  SQLLEN indicators[num_rows] = {0, 0, 0, 0, 0};
  // Ignore the 2nd and 4th sets (20 and 40); 10/30/50 are inserted.
  SQLUSMALLINT param_ops[num_rows] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED, SQL_PARAM_IGNORE,
                                      SQL_PARAM_PROCEED};
  // Pre-fill with a non-zero sentinel so an entry left unwritten (or set to
  // SQL_PARAM_ERROR) is distinguishable from SQL_PARAM_SUCCESS (which is 0).
  SQLUSMALLINT param_status[num_rows];
  memset(param_status, 0xFF, sizeof(param_status));
  SQLULEN params_processed = 0;

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, param_ops, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);

  // Bind with an explicit BufferLength (sizeof) so column-wise striding is
  // unambiguous — this test targets SQL_PARAM_IGNORE semantics, not the
  // BufferLength=0 fixed-size stride path (that is covered by SNOW-3720841).
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ids, sizeof(SQLBIGINT),
                         indicators);
  REQUIRE_ODBC(ret, stmt);

  // When 5 sets {10, 20, 30, 40, 50} are inserted with the 2nd and 4th marked SQL_PARAM_IGNORE
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(("INSERT INTO " + table.name() + " VALUES (?)").c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQL_ATTR_PARAMS_PROCESSED_PTR reports all 5 sets and the status array marks ignored sets SQL_PARAM_UNUSED
  CHECK(params_processed == num_rows);
  CHECK(param_status[0] == SQL_PARAM_SUCCESS);
  CHECK(param_status[1] == SQL_PARAM_UNUSED);
  CHECK(param_status[2] == SQL_PARAM_SUCCESS);
  CHECK(param_status[3] == SQL_PARAM_UNUSED);
  CHECK(param_status[4] == SQL_PARAM_SUCCESS);

  // And Query "SELECT id FROM {table} ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id FROM " + table.name() + " ORDER BY id");
  // Then Result should contain only the proceeded rows [10, 30, 50]
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 10);
  REQUIRE(SQLFetch(verify.getHandle()) == SQL_SUCCESS);
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 30);
  REQUIRE(SQLFetch(verify.getHandle()) == SQL_SUCCESS);
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 50);
  CHECK(SQLFetch(verify.getHandle()) == SQL_NO_DATA);
}

// SNOW-3235553: SQL_PARAM_IGNORE must be honored during array execution even
// when the application supplies an explicit SQL_ATTR_APP_PARAM_DESC. The binding
// path reads the *effective* APD, so PARAM_OPERATION_PTR must land there — before
// the effective-APD routing fix it was written to the inactive implicit APD, so
// the ignore array was dropped and every set (including 20/40) was inserted.
TEST_CASE_METHOD(ConnSchemaFixture, "should skip SQL_PARAM_IGNORE sets with an explicit APP_PARAM_DESC",
                 "[query][large_bindings][param_operation_ptr]") {
  // Given Snowflake client is logged in
  // And A temporary table with an id column exists
  ScopedTable table(conn, "lb_param_ignore_explicit", "id BIGINT");

  // And An explicit SQL_ATTR_APP_PARAM_DESC is assigned to the statement
  auto stmt = conn.createStatement();
  // RAII: the descriptor is freed on scope exit even if an assertion below
  // throws; per the ODBC spec, freeing an explicit descriptor reverts the
  // statement to its implicit APD, so no manual reset/free is needed.
  HandleWrapper explicit_apd(conn.handleWrapper().getHandle(), SQL_HANDLE_DESC);
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, explicit_apd.getHandle(), 0);
  REQUIRE_ODBC(ret, stmt);

  constexpr SQLULEN num_rows = 5;
  SQLBIGINT ids[num_rows] = {10, 20, 30, 40, 50};
  SQLLEN indicators[num_rows] = {0, 0, 0, 0, 0};
  // Ignore the 2nd and 4th sets (20 and 40); 10/30/50 are inserted.
  SQLUSMALLINT param_ops[num_rows] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED, SQL_PARAM_IGNORE,
                                      SQL_PARAM_PROCEED};
  // Pre-fill with a non-zero sentinel so an entry left unwritten (or set to
  // SQL_PARAM_ERROR) is distinguishable from SQL_PARAM_SUCCESS (which is 0).
  SQLUSMALLINT param_status[num_rows];
  memset(param_status, 0xFF, sizeof(param_status));
  SQLULEN params_processed = 0;

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, param_ops, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ids, sizeof(SQLBIGINT),
                         indicators);
  REQUIRE_ODBC(ret, stmt);

  // When 5 sets {10, 20, 30, 40, 50} are inserted with the 2nd and 4th marked SQL_PARAM_IGNORE
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(("INSERT INTO " + table.name() + " VALUES (?)").c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQL_ATTR_PARAMS_PROCESSED_PTR reports all 5 sets and the status array marks ignored sets SQL_PARAM_UNUSED
  CHECK(params_processed == num_rows);
  CHECK(param_status[0] == SQL_PARAM_SUCCESS);
  CHECK(param_status[1] == SQL_PARAM_UNUSED);
  CHECK(param_status[2] == SQL_PARAM_SUCCESS);
  CHECK(param_status[3] == SQL_PARAM_UNUSED);
  CHECK(param_status[4] == SQL_PARAM_SUCCESS);

  // And Query "SELECT id FROM {table} ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id FROM " + table.name() + " ORDER BY id");
  // Then Result should contain only the proceeded rows [10, 30, 50]
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 10);
  REQUIRE(SQLFetch(verify.getHandle()) == SQL_SUCCESS);
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 30);
  REQUIRE(SQLFetch(verify.getHandle()) == SQL_SUCCESS);
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 50);
  CHECK(SQLFetch(verify.getHandle()) == SQL_NO_DATA);
}
