#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_slong(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQLINTEGER& value, SQLLEN& indicator) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &value, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

void bind_double(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQLDOUBLE& value, SQLLEN& indicator) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &value, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

void bind_bit(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQLCHAR& value, SQLLEN& indicator) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &value, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

void bind_varchar(const StatementHandleWrapper& stmt, SQLUSMALLINT param, char* value, SQLLEN& indicator) {
  const auto len = static_cast<SQLLEN>(std::strlen(value));
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, value,
                                   len + 1, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

void bind_null_varchar(const StatementHandleWrapper& stmt, SQLUSMALLINT param, SQLLEN& indicator) {
  indicator = SQL_NULL_DATA;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), param, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);
}

void exec_direct(const StatementHandleWrapper& stmt, const char* sql) {
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
}

void exec_prepare(const StatementHandleWrapper& stmt, const char* sql) {
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
}

void exec_execute(const StatementHandleWrapper& stmt) {
  SQLRETURN ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

void fetch_one(const StatementHandleWrapper& stmt) {
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

std::string fetch_utf8(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  SQLCHAR buffer[256];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator >= 0);
  return {reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)};
}

}  // namespace

TEST_CASE("should bind basic types with positional parameters", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER i = 42;
  SQLDOUBLE d = 3.14;
  char hello[] = "hello";
  SQLCHAR flag = 1;
  SQLLEN i_ind = 0;
  SQLLEN d_ind = 0;
  SQLLEN hello_ind = SQL_NTS;
  SQLLEN flag_ind = 0;
  SQLLEN null_ind = SQL_NULL_DATA;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, i, i_ind);
  bind_double(stmt, 2, d, d_ind);
  bind_varchar(stmt, 3, hello, hello_ind);
  bind_bit(stmt, 4, flag, flag_ind);
  bind_null_varchar(stmt, 5, null_ind);

  // When Query "SELECT ?, ?, ?, ?, ?" is executed with positional parameters [42, 3.14, "hello", True, None]
  exec_direct(stmt, "SELECT ?, ?, ?, ?, ?");
  fetch_one(stmt);

  // Then Result should contain values matching the bound parameters
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 42);
  REQUIRE(get_data<SQL_C_DOUBLE>(stmt, 2) == Catch::Approx(3.14).epsilon(0.0001));
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 3) == "hello");
  REQUIRE(get_data<SQL_C_BIT>(stmt, 4) == 1);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 5) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert single row with parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR, active BOOLEAN) exists
  conn.execute("CREATE TEMPORARY TABLE bind_single (id NUMBER, name VARCHAR, active BOOLEAN)");

  SQLINTEGER id = 1;
  char name[] = "Alice";
  SQLCHAR active = 1;
  SQLLEN id_ind = 0;
  SQLLEN name_ind = SQL_NTS;
  SQLLEN active_ind = 0;
  const auto insert_stmt = conn.createStatement();
  bind_slong(insert_stmt, 1, id, id_ind);
  bind_varchar(insert_stmt, 2, name, name_ind);
  bind_bit(insert_stmt, 3, active, active_ind);

  // When Row with values [1, "Alice", True] is inserted using parameter binding
  exec_direct(insert_stmt, "INSERT INTO bind_single VALUES (?, ?, ?)");

  // And Query "SELECT * FROM table" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM bind_single");

  // Then Result should contain the inserted row [1, "Alice", True]
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Alice");
  REQUIRE(get_data<SQL_C_BIT>(stmt, 3) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert multiple rows sequentially with parameter binding",
                 "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_seq (id NUMBER, name VARCHAR)");

  SQLINTEGER id = 1;
  char alice[] = "Alice";
  char bob[] = "Bob";
  char charlie[] = "Charlie";
  SQLLEN id_ind = 0;
  SQLLEN name_ind = SQL_NTS;
  const auto insert_stmt = conn.createStatement();
  exec_prepare(insert_stmt, "INSERT INTO bind_seq VALUES (?, ?)");
  bind_slong(insert_stmt, 1, id, id_ind);

  // When Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"] are inserted sequentially using parameter binding
  bind_varchar(insert_stmt, 2, alice, name_ind);
  exec_execute(insert_stmt);
  id = 2;
  bind_varchar(insert_stmt, 2, bob, name_ind);
  exec_execute(insert_stmt);
  id = 3;
  bind_varchar(insert_stmt, 2, charlie, name_ind);
  exec_execute(insert_stmt);

  // And Query "SELECT * FROM table ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM bind_seq ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 3 rows with correct values
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Alice");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Bob");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 3);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Charlie");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should update row with parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_upd (id NUMBER, name VARCHAR)");

  // And Row [1, "Alice"] is inserted
  conn.execute("INSERT INTO bind_upd VALUES (1, 'Alice')");

  char updated[] = "Alice Updated";
  SQLINTEGER id = 1;
  SQLLEN name_ind = SQL_NTS;
  SQLLEN id_ind = 0;
  const auto update_stmt = conn.createStatement();
  bind_varchar(update_stmt, 1, updated, name_ind);
  bind_slong(update_stmt, 2, id, id_ind);

  // When Query "UPDATE table SET name = ? WHERE id = ?" is executed with parameters ["Alice Updated", 1]
  exec_direct(update_stmt, "UPDATE bind_upd SET name = ? WHERE id = ?");

  // And Query "SELECT * FROM table" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM bind_upd");

  // Then Result should contain [1, "Alice Updated"]
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Alice Updated");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should delete row with parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_del (id NUMBER, name VARCHAR)");

  // And Rows [1, "Alice"] and [2, "Bob"] are inserted
  conn.execute("INSERT INTO bind_del VALUES (1, 'Alice'), (2, 'Bob')");

  SQLINTEGER id = 1;
  SQLLEN id_ind = 0;
  const auto delete_stmt = conn.createStatement();
  bind_slong(delete_stmt, 1, id, id_ind);

  // When Query "DELETE FROM table WHERE id = ?" is executed with parameter [1]
  exec_direct(delete_stmt, "DELETE FROM bind_del WHERE id = ?");

  // And Query "SELECT * FROM table" is executed
  const auto stmt = conn.execute_fetch("SELECT * FROM bind_del");

  // Then Result should contain only [2, "Bob"]
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Bob");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should select with WHERE clause parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR, age NUMBER) exists
  conn.execute("CREATE TEMPORARY TABLE bind_where (id NUMBER, name VARCHAR, age NUMBER)");

  // And Rows [1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35] are inserted
  conn.execute("INSERT INTO bind_where VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35)");

  SQLINTEGER age = 28;
  SQLLEN age_ind = 0;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, age, age_ind);

  // When Query "SELECT * FROM table WHERE age > ?" is executed with parameter [28]
  exec_direct(stmt, "SELECT * FROM bind_where WHERE age > ?");

  // Then Result should contain rows for "Alice" and "Charlie"
  std::vector<std::string> names;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_ODBC(ret, stmt);
    names.push_back(get_data<SQL_C_CHAR>(stmt, 2));
  }
  REQUIRE(names.size() == 2);
  REQUIRE((names[0] == "Alice" || names[1] == "Alice"));
  REQUIRE((names[0] == "Charlie" || names[1] == "Charlie"));
}

TEST_CASE("should handle NULL values in parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLLEN n1 = SQL_NULL_DATA;
  SQLINTEGER i = 42;
  SQLLEN i_ind = 0;
  SQLLEN n3 = SQL_NULL_DATA;
  const auto stmt = conn.createStatement();
  bind_null_varchar(stmt, 1, n1);
  bind_slong(stmt, 2, i, i_ind);
  bind_null_varchar(stmt, 3, n3);

  // When Query "SELECT ?, ?, ?" is executed with parameters [None, 42, None]
  exec_direct(stmt, "SELECT ?, ?, ?");
  fetch_one(stmt);

  // Then Result should contain [NULL, 42, NULL]
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 2) == 42);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 3) == std::nullopt);
}

TEST_CASE("should handle special characters in string binding", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  char injection[] = "'; DROP TABLE test; --";
  char xss[] = "<script>alert('xss')</script>";
  char newlines[] = "Line1\nLine2\nLine3";
  char tabs[] = "Tab\t\tSeparated\t\tValues";
  char quotes[] = "Quote'Within\"String";
  char escaped[] = "\\n\\t\\r\\\\";
  const char* expected[] = {injection, xss, newlines, tabs, quotes, escaped};
  char* values[] = {injection, xss, newlines, tabs, quotes, escaped};

  // When Query "SELECT ?::VARCHAR" is executed with parameter containing special characters
  for (size_t i = 0; i < 6; ++i) {
    INFO(expected[i]);
    SQLLEN ind = SQL_NTS;
    const auto stmt = conn.createStatement();
    bind_varchar(stmt, 1, values[i], ind);
    exec_direct(stmt, "SELECT ?::VARCHAR");
    fetch_one(stmt);

    // Then Result should contain the exact special character string
    REQUIRE(get_data<SQL_C_CHAR>(stmt, 1) == expected[i]);
  }
}

TEST_CASE("should handle Unicode characters in parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;
  conn.execute("CREATE TEMPORARY TABLE unicode_bind (a VARCHAR, b VARCHAR)");

  std::string japanese = "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e";
  std::string snowman = "\xe2\x9b\x84";
  SQLLEN j_ind = static_cast<SQLLEN>(japanese.size());
  SQLLEN s_ind = static_cast<SQLLEN>(snowman.size());
  const auto stmt = conn.createStatement();
  exec_prepare(stmt, "INSERT INTO unicode_bind SELECT TO_VARCHAR(?::BINARY, 'UTF-8'), TO_VARCHAR(?::BINARY, 'UTF-8')");
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, japanese.size(),
                                   0, japanese.data(), j_ind, &j_ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARBINARY, snowman.size(), 0,
                         snowman.data(), s_ind, &s_ind);
  REQUIRE_ODBC(ret, stmt);
  exec_execute(stmt);

  // When Query "SELECT ?::VARCHAR, ?::VARCHAR" is executed with parameters ["日本語", "⛄"]
  const auto fetch = conn.execute_fetch("SELECT a, b FROM unicode_bind");

  // Then Result should contain Unicode strings ["日本語", "⛄"]
  REQUIRE(fetch_utf8(fetch, 1) == japanese);
  REQUIRE(fetch_utf8(fetch, 2) == snowman);
}

TEST_CASE("should bind zero values", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER zero = 0;
  SQLDOUBLE zd = 0.0;
  char empty[] = "";
  SQLLEN z_ind = 0;
  SQLLEN zd_ind = 0;
  SQLLEN empty_ind = SQL_NTS;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, zero, z_ind);
  bind_double(stmt, 2, zd, zd_ind);
  bind_varchar(stmt, 3, empty, empty_ind);

  // When Query "SELECT ?, ?::FLOAT, ?::VARCHAR" is executed with parameters [0, 0.0, ""]
  exec_direct(stmt, "SELECT ?, ?::FLOAT, ?::VARCHAR");
  fetch_one(stmt);

  // Then Result should contain zero and empty values [0, 0.0, ""]
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 0);
  REQUIRE(get_data<SQL_C_DOUBLE>(stmt, 2) == Catch::Approx(0.0));
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 3) == "");
}

TEST_CASE("should handle mixed type casting with parameter binding", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER n = 42;
  char hello[] = "hello";
  SQLCHAR flag = 1;
  SQLLEN n_ind = 0;
  SQLLEN hello_ind = SQL_NTS;
  SQLLEN flag_ind = 0;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, n, n_ind);
  bind_varchar(stmt, 2, hello, hello_ind);
  bind_bit(stmt, 3, flag, flag_ind);

  // When Query "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN" is executed with parameters [42, "hello", True]
  exec_direct(stmt, "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN");
  fetch_one(stmt);

  // Then Result should match the type-casted parameters [42, "hello", True]
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 42);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "hello");
  REQUIRE(get_data<SQL_C_BIT>(stmt, 3) == 1);
}

TEST_CASE("should silently ignore extra positional parameters", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER a = 1;
  SQLINTEGER b = 2;
  SQLINTEGER c = 3;
  SQLLEN a_ind = 0;
  SQLLEN b_ind = 0;
  SQLLEN c_ind = 0;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, a, a_ind);
  bind_slong(stmt, 2, b, b_ind);
  bind_slong(stmt, 3, c, c_ind);

  // When Query with 2 placeholders is executed with 3 arguments
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?, ?"), SQL_NTS);

  // Then Query should successfully execute, ignoring the extra argument
  REQUIRE_ODBC(ret, stmt);
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 2) == 2);
}

TEST_CASE("should raise error for too few positional parameters", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER a = 1;
  SQLLEN a_ind = 0;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, a, a_ind);

  // When Query with 3 placeholders is executed with 1 argument
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?, ?, ?"), SQL_NTS);

  // Then Error should be raised for the unbound placeholders
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("42601"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should insert multiple rows using multirow binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_multi (id NUMBER, name VARCHAR)");

  constexpr SQLULEN num_rows = 3;
  SQLINTEGER ids[num_rows] = {1, 2, 3};
  char names[num_rows][16] = {"Alice", "Bob", "Charlie"};
  SQLLEN id_inds[num_rows] = {0, 0, 0};
  SQLLEN name_inds[num_rows] = {SQL_NTS, SQL_NTS, SQL_NTS};
  SQLUSMALLINT param_status[num_rows] = {};
  SQLULEN params_processed = 0;
  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, ids, 0, id_inds);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 15, 0, names[0], 16,
                         name_inds);
  REQUIRE_ODBC(ret, insert_stmt);

  // When Rows [[1, "Alice"], [2, "Bob"], [3, "Charlie"]] are inserted using multirow binding
  exec_direct(insert_stmt, "INSERT INTO bind_multi VALUES (?, ?)");
  REQUIRE(params_processed == num_rows);

  // And Query "SELECT * FROM table ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM bind_multi ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain 3 rows with correct values
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Alice");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Bob");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 3);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Charlie");
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL values in multirow binding", "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, value VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_nulls (id NUMBER, value VARCHAR)");

  constexpr SQLULEN num_rows = 3;
  SQLINTEGER ids[num_rows] = {1, 2, 3};
  char values[num_rows][16] = {"", "value", ""};
  SQLLEN id_inds[num_rows] = {0, 0, 0};
  SQLLEN value_inds[num_rows] = {SQL_NULL_DATA, SQL_NTS, SQL_NULL_DATA};
  SQLUSMALLINT param_status[num_rows] = {};
  SQLULEN params_processed = 0;
  const auto insert_stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, param_status, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLSetStmtAttr(insert_stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, ids, 0, id_inds);
  REQUIRE_ODBC(ret, insert_stmt);
  ret = SQLBindParameter(insert_stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 15, 0, values[0], 16,
                         value_inds);
  REQUIRE_ODBC(ret, insert_stmt);

  // When Rows [[1, NULL], [2, "value"], [3, NULL]] are inserted using multirow binding
  exec_direct(insert_stmt, "INSERT INTO bind_nulls VALUES (?, ?)");

  // And Query "SELECT * FROM table ORDER BY id" is executed
  const auto stmt = conn.createStatement();
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM bind_nulls ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [[1, NULL], [2, "value"], [3, NULL]]
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 1);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 2);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "value");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_SLONG>(stmt, 1) == 3);
  REQUIRE(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE("should bind many parameters", "[parameter_binding]") {
  // Given Snowflake client is logged in
  Connection conn;

  SQLINTEGER values[20];
  SQLLEN inds[20];
  const auto stmt = conn.createStatement();
  for (int i = 0; i < 20; ++i) {
    values[i] = i;
    inds[i] = 0;
    bind_slong(stmt, static_cast<SQLUSMALLINT>(i + 1), values[i], inds[i]);
  }

  // When Query with 20 positional parameters is executed with values [0..19]
  exec_direct(stmt, "SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?");
  fetch_one(stmt);

  // Then Result should contain all 20 values in order
  for (int i = 0; i < 20; ++i) {
    REQUIRE(get_data<SQL_C_SLONG>(stmt, static_cast<SQLUSMALLINT>(i + 1)) == i);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind parameters with OR clause for multiple value matching",
                 "[parameter_binding]") {
  // Given Snowflake client is logged in

  // And A temporary table with columns (id NUMBER, name VARCHAR) exists
  conn.execute("CREATE TEMPORARY TABLE bind_or (id NUMBER, name VARCHAR)");

  // And Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"], [4, "David"], [5, "Eve"] are inserted
  conn.execute("INSERT INTO bind_or VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie'), (4, 'David'), (5, 'Eve')");

  SQLINTEGER a = 1;
  SQLINTEGER b = 3;
  SQLINTEGER c = 5;
  SQLLEN a_ind = 0;
  SQLLEN b_ind = 0;
  SQLLEN c_ind = 0;
  const auto stmt = conn.createStatement();
  bind_slong(stmt, 1, a, a_ind);
  bind_slong(stmt, 2, b, b_ind);
  bind_slong(stmt, 3, c, c_ind);

  // When Query "SELECT FROM {table_name} WHERE id = ? OR id = ? OR id = ? ORDER BY id" is executed with parameters [1,
  // 3, 5]
  exec_direct(stmt, "SELECT * FROM bind_or WHERE id = ? OR id = ? OR id = ? ORDER BY id");

  // Then Result should contain [("Alice"), ("Charlie"), ("Eve")]
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Alice");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Charlie");
  fetch_one(stmt);
  REQUIRE(get_data<SQL_C_CHAR>(stmt, 2) == "Eve");
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
