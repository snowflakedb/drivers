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

// Bulk-inserts `count` rows into `table`(id, n, f, flag, txt) via column-wise
// ODBC array binding.
// Row i: id=i, n=NULL if i%7==0 else i*10, f=i*0.5, flag=i%2==0, txt="txt-"+str(i).
// 13200 rows × 5 cols = 66000 cells, above the default 65280 threshold.
// Returns the query ID of the INSERT via SQL_SF_STMT_ATTR_LAST_QUERY_ID.
static std::string bulk_insert_types(Connection& conn, const std::string& table, int count) {
  constexpr int TXT_BUF = 24;
  std::vector<SQLBIGINT> ids(count), ns(count);
  std::vector<double> fs(count);
  std::vector<SQLCHAR> flags(count);
  std::vector<char> txts(static_cast<size_t>(count) * TXT_BUF, '\0');
  std::vector<SQLLEN> id_inds(count, 0), n_inds(count), f_inds(count, 0), flag_inds(count, 0), txt_inds(count);
  std::vector<SQLUSMALLINT> param_status(count, 0);
  SQLULEN params_processed = 0;

  for (int i = 0; i < count; i++) {
    ids[i] = static_cast<SQLBIGINT>(i);
    if (i % 7 == 0) {
      ns[i] = 0;
      n_inds[i] = SQL_NULL_DATA;
    } else {
      ns[i] = static_cast<SQLBIGINT>(i) * 10;
      n_inds[i] = 0;
    }
    fs[i] = i * 0.5;
    flags[i] = static_cast<SQLCHAR>(i % 2 == 0 ? 1 : 0);
    std::string txt = "txt-" + std::to_string(i);
    std::strncpy(&txts[static_cast<size_t>(i) * TXT_BUF], txt.c_str(), TXT_BUF - 1);
    txt_inds[i] = static_cast<SQLLEN>(txt.size());
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
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIGINT, 0, 0, ns.data(),
                         sizeof(SQLBIGINT), n_inds.data());
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

  const std::string sql = "INSERT INTO " + table + " VALUES (?, ?, ?, ?, ?)";
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

// Returns the RFC-4180 hazard string for row i (7-cycle rotation):
//   0 – comma, 1 – double-quote, 2 – newline, 3 – backslash,
//   4 – empty string, 5 – NULL (indicator set by caller), 6 – UTF-8 multibyte.
static std::string make_hazard_string(int i) {
  switch (i % 7) {
    case 0:
      return "val," + std::to_string(i);
    case 1:
      return "say\"" + std::to_string(i) + "\"";
    case 2:
      return "a\nb";
    case 3:
      return "C:\\dir\\" + std::to_string(i);
    case 4:
      return "";
    case 5:
      return "";  // NULL — indicator must be SQL_NULL_DATA
    case 6:
      return "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e";  // 日本語
  }
  return "";
}

// Bulk-inserts `count` rows into `table`(id BIGINT, txt VARCHAR) with hazard
// strings via column-wise ODBC array binding.
// 33000 rows × 2 cols = 66000 cells, above the default 65280 threshold.
// Returns the query ID of the INSERT via SQL_SF_STMT_ATTR_LAST_QUERY_ID.
static std::string bulk_insert_hazard_strings(Connection& conn, const std::string& table, int count) {
  constexpr int TXT_BUF = 128;
  std::vector<SQLBIGINT> ids(count);
  std::vector<char> txts(static_cast<size_t>(count) * TXT_BUF, '\0');
  std::vector<SQLLEN> id_inds(count, 0), txt_inds(count);
  std::vector<SQLUSMALLINT> param_status(count, 0);
  SQLULEN params_processed = 0;

  for (int i = 0; i < count; i++) {
    ids[i] = static_cast<SQLBIGINT>(i);
    if (i % 7 == 5) {
      txt_inds[i] = SQL_NULL_DATA;
    } else {
      std::string s = make_hazard_string(i);
      std::strncpy(&txts[static_cast<size_t>(i) * TXT_BUF], s.c_str(), TXT_BUF - 1);
      txt_inds[i] = static_cast<SQLLEN>(s.size());
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

  // And A temporary table with columns (id NUMBER, n NUMBER, f FLOAT, flag BOOLEAN, txt VARCHAR) exists
  ScopedTable table(conn, "lb_types", "id BIGINT, n BIGINT, f DOUBLE, flag BOOLEAN, txt VARCHAR");

  // When 13200 rows are inserted using multirow binding
  auto before = list_system_bind_file_count(conn);
  std::string qid = bulk_insert_types(conn, table.name(), 13200);

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  CHECK(after > before);

  // And Query "SELECT id, n, f, flag, txt FROM {table} ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, n, f, flag, txt FROM " + table.name() +
                                   " WHERE id IN (0, 1, 7, 100, 13199) ORDER BY id");

  // Then Result should contain the same values as the bound parameters
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 0);  // row 0: n=NULL (0%7=0), flag=TRUE
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 2) == std::nullopt);
  CHECK(get_data<SQL_C_DOUBLE>(verify, 3) == Catch::Approx(0.0));
  CHECK(get_data<SQL_C_BIT>(verify, 4) == 1);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 5) == "txt-0");

  SQLRETURN ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 1: n=10, f=0.5, flag=FALSE
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 1);
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 2) == 10LL);
  CHECK(get_data<SQL_C_DOUBLE>(verify, 3) == Catch::Approx(0.5));
  CHECK(get_data<SQL_C_BIT>(verify, 4) == 0);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 5) == "txt-1");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 7: n=NULL (7%7=0), f=3.5, flag=FALSE
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 7);
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 2) == std::nullopt);
  CHECK(get_data<SQL_C_DOUBLE>(verify, 3) == Catch::Approx(3.5));
  CHECK(get_data<SQL_C_BIT>(verify, 4) == 0);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 5) == "txt-7");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 100: n=1000, f=50.0, flag=TRUE
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 100);
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 2) == 1000LL);
  CHECK(get_data<SQL_C_DOUBLE>(verify, 3) == Catch::Approx(50.0));
  CHECK(get_data<SQL_C_BIT>(verify, 4) == 1);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 5) == "txt-100");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 13199: n=131990, f=6599.5, flag=FALSE
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 13199);
  CHECK(get_data_optional<SQL_C_SBIGINT>(verify, 2) == 131990LL);
  CHECK(get_data<SQL_C_DOUBLE>(verify, 3) == Catch::Approx(6599.5));
  CHECK(get_data<SQL_C_BIT>(verify, 4) == 0);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 5) == "txt-13199");

  ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should preserve CSV escaping hazards via stage binding",
                 "[query][large_bindings]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, txt VARCHAR) exists
  ScopedTable table(conn, "lb_hazards", "id BIGINT, txt VARCHAR");

  // When 33000 rows are inserted using multirow binding with values cycling every 7 rows through [[0, "val,0"], [1,
  // "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL], [6, "日本語"]]
  auto before = list_system_bind_file_count(conn);
  std::string qid = bulk_insert_hazard_strings(conn, table.name(), 33000);

  // Then the bind file on SYSTEM$BIND from the last bulk insert should contain the same values as the bound parameters
  auto after = list_system_bind_file_count(conn);
  INFO("INSERT query_id: " << qid);
  CHECK(after > before);

  // And Query "SELECT id, txt FROM {table} WHERE id BETWEEN 0 AND 6 ORDER BY id" is executed
  auto verify = conn.execute_fetch("SELECT id, txt FROM " + table.name() + " WHERE id BETWEEN 0 AND 6 ORDER BY id");

  // Then Result should contain rows [[0, "val,0"], [1, "say\"1\""], [2, "a\nb"], [3, "C:\\dir\\3"], [4, ""], [5, NULL],
  // [6, "日本語"]]
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 0);  // row 0: comma
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == "val,0");

  SQLRETURN ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 1: double-quote
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 1);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == "say\"1\"");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 2: newline
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 2);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == "a\nb");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 3: backslash
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 3);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == "C:\\dir\\3");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 4: empty string (distinct from NULL)
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 4);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == "");

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  // row 5: NULL
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 5);
  CHECK(get_data_optional<SQL_C_CHAR>(verify, 2) == std::nullopt);

  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  CHECK(get_data<SQL_C_SBIGINT>(verify, 1) == 6);
  {
    SQLCHAR buffer[128];
    memset(buffer, 0xFF, sizeof(buffer));
    SQLLEN indicator = 0;
    ret = SQLGetData(verify.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
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
  }

  ret = SQLFetch(verify.getHandle());
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
  SKIP_OLD_DRIVER("BD#78", "Old driver does the comparison with > instead of >=");
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
  CHECK(after > before);

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
