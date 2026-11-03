#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "WideString.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_VARCHAR and read back",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_CHAR "hello world" is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "hello world";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 200, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as "hello world"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "hello world");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR empty string to SQL_VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_CHAR empty string is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 200, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as empty
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR at max length to fixed-size VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(5))");

  // When SQL_C_CHAR "abcde" is bound to VARCHAR(5) and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "abcde";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 5, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as "abcde"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "abcde");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR exceeding fixed-size VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(5))");

  // When SQL_C_CHAR "hello world" is bound to VARCHAR(5) and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "hello world";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 5, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the insert is rejected with SQL_ERROR and a server-supplied
  // SQLSTATE (one of HY000, 22000, or 22001 — see below for why)
  //
  // The server normally returns SQLSTATE 22000 (generic data exception)
  // for this scenario, which the driver forwards verbatim. Two further
  // outcomes are also acceptable: if the server omits sqlState entirely
  // the driver falls back to HY000; if it omits sqlState but supplies
  // Snowflake error code 100078, sf_core::sql_state_from_code recovers
  // 22001. We accept any of these three rather than promoting a
  // specific value client-side.
  CHECK(ret == SQL_ERROR);
  std::string sqlstate = get_sqlstate(stmt);
  CHECK((sqlstate == "HY000" || sqlstate == "22000" || sqlstate == "22001"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR to SQL_VARCHAR and read back",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_WCHAR "test" is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'t', 'e', 's', 't', 0};
  SQLLEN ind = SQL_NTS;
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 200, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as "test"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "test");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR empty string to SQL_VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_WCHAR empty string is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {0};
  SQLLEN ind = SQL_NTS;
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 200, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as empty
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 200, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR with NULL indicator to SQL_VARCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_WCHAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 200, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT to wide SQL types as SQL_C_WCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a wide string is bound with SQL_C_DEFAULT to SQL_WCHAR, SQL_WVARCHAR and SQL_WLONGVARCHAR and inserted
  const std::pair<SQLSMALLINT, std::u32string> cases[] = {
      {SQL_WCHAR, U"wchar"},
      {SQL_WVARCHAR, U"wvarchar"},
      {SQL_WLONGVARCHAR, U"wlongvarchar"},
  };
  for (const auto& [sql_type, text] : cases) {
    INFO("ParameterType " << sql_type);
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    std::vector<SQLWCHAR> val = sf::wide::encode_wide(text);
    SQLLEN ind = SQL_NTS;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, sql_type, 200, 0, val.data(),
                           static_cast<SQLLEN>(val.size() * sizeof(SQLWCHAR)), &ind);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // Then every value is read back in full
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t ORDER BY col");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "wchar");
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "wlongvarchar");
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "wvarchar");
  REQUIRE_ODBC_NO_DATA(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT parameter array to SQL_WVARCHAR as SQL_C_WCHAR",
                 "[e2e][types][c_char][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a column-wise array of two-character wide strings is bound with SQL_C_DEFAULT and SQL_WVARCHAR
  constexpr SQLULEN num_rows = 3;
  constexpr SQLLEN row_chars = 2;
  std::vector<SQLWCHAR> values = sf::wide::encode_wide(U"abcdef");
  SQLLEN indicators[num_rows] = {row_chars * sizeof(SQLWCHAR), row_chars * sizeof(SQLWCHAR),
                                 row_chars * sizeof(SQLWCHAR)};
  SQLULEN params_processed = 0;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_WVARCHAR, 200, 0, values.data(),
                         row_chars * sizeof(SQLWCHAR), indicators);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == num_rows);

  // Then every row is read back in full
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t ORDER BY col");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "ab");
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "cd");
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "ef");
  REQUIRE_ODBC_NO_DATA(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
}
