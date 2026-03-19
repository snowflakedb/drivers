#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"

// ============================================================================
// SQL_ATTR_PARAMSET_SIZE
// ============================================================================

TEST_CASE("SQL_ATTR_PARAMSET_SIZE default value is 1.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is queried on a fresh statement
  SQLULEN value = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value 1
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 1);
}

TEST_CASE("SQL_ATTR_PARAMSET_SIZE can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set to 5
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)5, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved value should be 5
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 5);
}

// ============================================================================
// SQL_ATTR_PARAM_BIND_TYPE
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_BIND_TYPE default value is SQL_PARAM_BIND_BY_COLUMN.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_TYPE is queried on a fresh statement
  SQLULEN value = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value 0
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == SQL_PARAM_BIND_BY_COLUMN);
}

TEST_CASE("SQL_ATTR_PARAM_BIND_TYPE can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_TYPE is set to a row size
  const SQLULEN row_size = 128;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, (SQLPOINTER)row_size, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved value should match
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == row_size);
}

// ============================================================================
// SQL_ATTR_PARAM_BIND_OFFSET_PTR
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_BIND_OFFSET_PTR default value is NULL.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_OFFSET_PTR is queried on a fresh statement
  SQLLEN* ptr = reinterpret_cast<SQLLEN*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value NULL
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_BIND_OFFSET_PTR can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_BIND_OFFSET_PTR is set to a pointer
  SQLLEN offset = 0;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &offset, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved pointer should match
  SQLLEN* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_OFFSET_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == &offset);
}

// ============================================================================
// SQL_ATTR_PARAM_STATUS_PTR
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_STATUS_PTR default value is NULL.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_STATUS_PTR is queried on a fresh statement
  SQLUSMALLINT* ptr = reinterpret_cast<SQLUSMALLINT*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value NULL
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_STATUS_PTR can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_STATUS_PTR is set to a pointer
  SQLUSMALLINT status[5] = {};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved pointer should match
  SQLUSMALLINT* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == status);
}

// ============================================================================
// SQL_ATTR_PARAMS_PROCESSED_PTR
// ============================================================================

TEST_CASE("SQL_ATTR_PARAMS_PROCESSED_PTR default value is NULL.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMS_PROCESSED_PTR is queried on a fresh statement
  SQLULEN* ptr = reinterpret_cast<SQLULEN*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value NULL
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAMS_PROCESSED_PTR can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMS_PROCESSED_PTR is set to a pointer
  SQLULEN processed = 0;
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved pointer should match
  SQLULEN* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == &processed);
}

// ============================================================================
// SQL_ATTR_PARAM_OPERATION_PTR
// ============================================================================

TEST_CASE("SQL_ATTR_PARAM_OPERATION_PTR default value is NULL.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_OPERATION_PTR is queried on a fresh statement
  SQLUSMALLINT* ptr = reinterpret_cast<SQLUSMALLINT*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value NULL
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_PARAM_OPERATION_PTR can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAM_OPERATION_PTR is set to a pointer
  SQLUSMALLINT ops[5] = {};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved pointer should match
  SQLUSMALLINT* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == ops);
}

// ============================================================================
// Array execution
// ============================================================================

TEST_CASE("PARAMSET_SIZE greater than 1 executes multiple parameter sets.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;

  const std::string table = "param_array_test_paramset_size";
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE " + table + " (v INTEGER)");

  auto stmt = conn.createStatement();

  // When SQLExecDirect is called with PARAMSET_SIZE set to 3 and an array of 3 integer values
  SQLINTEGER values[3] = {10, 20, 30};
  SQLLEN indicators[3] = {0, 0, 0};

  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                                   sizeof(SQLINTEGER), indicators);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)("INSERT INTO " + table + " VALUES (?)").c_str(), SQL_NTS);

  // Then it should return SQL_SUCCESS and insert all 3 rows
  REQUIRE(ret == SQL_SUCCESS);

  // Verify 3 rows were inserted
  auto verify_stmt = conn.execute("SELECT COUNT(*) FROM " + table);
  SQLLEN count = 0;
  SQLBindCol(verify_stmt.getHandle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  SQLFetch(verify_stmt.getHandle());
  CHECK(count == 3);

  conn.execute("DROP TABLE IF EXISTS " + table);
}

TEST_CASE("PARAMS_PROCESSED_PTR is written with the number of parameter sets after execution.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;

  const std::string table = "param_array_test_params_processed";
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE " + table + " (v INTEGER)");

  auto stmt = conn.createStatement();

  SQLINTEGER values[3] = {1, 2, 3};
  SQLLEN indicators[3] = {0, 0, 0};
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                                   sizeof(SQLINTEGER), indicators);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLExecDirect is called with PARAMSET_SIZE set to 3 and PARAMS_PROCESSED_PTR bound
  SQLULEN processed = 0;
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &processed, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)("INSERT INTO " + table + " VALUES (?)").c_str(), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then PARAMS_PROCESSED_PTR should contain 3 after execution
  CHECK(processed == 3);

  conn.execute("DROP TABLE IF EXISTS " + table);
}

TEST_CASE("PARAM_STATUS_PTR is written with SQL_PARAM_SUCCESS for each row after execution.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Parameter array statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;

  const std::string table = "param_array_test_param_status";
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE " + table + " (v INTEGER)");

  auto stmt = conn.createStatement();

  SQLINTEGER values[3] = {7, 8, 9};
  SQLLEN indicators[3] = {0, 0, 0};
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, values,
                                   sizeof(SQLINTEGER), indicators);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLExecDirect is called with PARAMSET_SIZE set to 3 and PARAM_STATUS_PTR bound
  SQLUSMALLINT status[3] = {0xFFFF, 0xFFFF, 0xFFFF};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_STATUS_PTR, status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)("INSERT INTO " + table + " VALUES (?)").c_str(), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then each slot of PARAM_STATUS_PTR should contain SQL_PARAM_SUCCESS after execution
  CHECK(status[0] == SQL_PARAM_SUCCESS);
  CHECK(status[1] == SQL_PARAM_SUCCESS);
  CHECK(status[2] == SQL_PARAM_SUCCESS);

  conn.execute("DROP TABLE IF EXISTS " + table);
}
