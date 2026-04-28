#include <cmath>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR float string to SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "3.14" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "3.14";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 3.14
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(3.14, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR integer string to SQL_REAL",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "100" is bound to SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "100";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_REAL, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 100.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(100.0, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_FLOAT synonym", "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "1.23" is bound to SQL_FLOAT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "1.23";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_FLOAT, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 1.23
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(1.23, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to FLOAT4 column", "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT4)");

  // When SQL_C_CHAR "5.5" is bound to SQL_DOUBLE and inserted into a FLOAT4 column
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "5.5";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 5.5
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(5.5, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to FLOAT8 column", "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT8)");

  // When SQL_C_CHAR "9.81" is bound to SQL_DOUBLE and inserted into a FLOAT8 column
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "9.81";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 9.81
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(9.81, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to DOUBLE PRECISION column",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DOUBLE PRECISION)");

  // When SQL_C_CHAR "2.22" is bound to SQL_DOUBLE and inserted into a DOUBLE PRECISION column
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "2.22";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 2.22
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(2.22, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to REAL column", "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col REAL)");

  // When SQL_C_CHAR "7.77" is bound to SQL_REAL and inserted into a REAL column
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "7.77";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_REAL, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 7.77
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(7.77, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR float string to SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR "2.71" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', '.', '7', '1', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as approximately 2.71
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(2.71, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR integer string to SQL_REAL",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR "200" is bound to SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', '0', '0', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_REAL, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 200.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(200.0, 0.001));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR with NULL indicator to SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}

// ============================================================================
// Non-finite IEEE-754 literals: "Infinity", "-Infinity", "NaN"
//
// Per the MS ODBC spec Appendix C, a string bound as SQL_C_CHAR / SQL_C_WCHAR
// to a numeric SQL target must match the "numeric-literal" grammar, which
// does not admit the tokens "Infinity", "-Infinity" or "NaN". The driver
// therefore rejects them client-side with SQLSTATE 22018 (Invalid character
// value for cast specification).
//
// This is a deliberate behavioral divergence from the old Snowflake driver,
// which forwards these strings to the server and stores them as non-finite
// FLOAT values. See BD#48. Applications that need to insert Infinity / NaN
// should bind SQL_C_DOUBLE or SQL_C_FLOAT instead (both are spec-permitted
// and supported by the new driver).
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR Infinity string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "Infinity" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "Infinity";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR negative Infinity string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "-Infinity" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "-Infinity";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR NaN string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_CHAR "NaN" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "NaN";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_WCHAR Infinity string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR "Infinity" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'I', 'n', 'f', 'i', 'n', 'i', 't', 'y', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_WCHAR negative Infinity string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR "-Infinity" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'-', 'I', 'n', 'f', 'i', 'n', 'i', 't', 'y', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_WCHAR NaN string for SQL_DOUBLE",
                 "[c_char][conversion][sql_real]") {
  SKIP_OLD_DRIVER("BD#48", "Old driver accepts non-finite string literals; new driver rejects per ODBC spec");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_C_WCHAR "NaN" is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'N', 'a', 'N', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then SQLExecute fails with SQLSTATE 22018
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}
