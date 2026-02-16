#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLBindParameter - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Binds integer input parameter",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_value = 42;
  SQLLEN indicator = 0;
  ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Binds string input parameter",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  char param_value[] = "hello";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(param_value), 0,
                         param_value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char result[64] = {};
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, result, sizeof(result), &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(result) == "hello");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Binds NULL parameter value",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN indicator = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, nullptr, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 999;
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(result_indicator == SQL_NULL_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Re-execute with different parameter value",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_value = 10;
  SQLLEN indicator = 0;
  ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // First execution
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 10);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Second execution with different value
  param_value = 20;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 20);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Multiple parameters",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ?, ?")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param1 = 100;
  SQLINTEGER param2 = 200;
  SQLLEN ind1 = 0, ind2 = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param1, 0, &ind1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLBindParameter(stmt_handle(), 2, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param2, 0, &ind2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result1 = 0, result2 = 0;
  SQLLEN rind1 = 0, rind2 = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result1, 0, &rind1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_SLONG, &result2, 0, &rind2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result1 == 100);
  REQUIRE(result2 == 200);
}

// ============================================================================
// SQLBindParameter - Error Cases
// ============================================================================

TEST_CASE("SQLBindParameter: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_value = 1;
  SQLLEN indicator = 0;
  const SQLRETURN ret =
      SQLBindParameter(SQL_NULL_HSTMT, 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: 07009 for parameter number 0",
                 "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_value = 1;
  SQLLEN indicator = 0;
  // 07009: Invalid descriptor index
  SQLRETURN ret =
      SQLBindParameter(stmt_handle(), 0, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: Rebinding same parameter number replaces binding",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param1 = 111;
  SQLLEN ind1 = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param1, 0, &ind1);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param2 = 222;
  SQLLEN ind2 = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param2, 0, &ind2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 222);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: HY009 for both null pointers on input parameter",
                 "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // HY009: Invalid argument value (both ParameterValuePtr and StrLen_or_IndPtr are null for input parameter)
  SQLRETURN ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, nullptr, 0, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: HY105 for invalid InputOutputType",
                 "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_value = 1;
  SQLLEN indicator = 0;
  // HY105: Invalid parameter type (999 is not a valid InputOutputType)
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, 999, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE_EXPECTED_ERROR(ret, "HY105", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: HY003 for invalid ValueType",
                 "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_value = 1;
  SQLLEN indicator = 0;
  // HY003: Invalid application buffer type (9999 is not a valid C data type)
  SQLRETURN ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, 9999, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE_EXPECTED_ERROR(ret, "HY003", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: HY004 for invalid ParameterType",
                 "[odbc-api][bindparameter][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_value = 1;
  SQLLEN indicator = 0;
  // HY004: Invalid SQL data type (8888 is not a valid SQL data type)
  SQLRETURN ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, 8888, 0, 0, &param_value, 0, &indicator);
  REQUIRE_EXPECTED_ERROR(ret, "HY004", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLBindParameter - Reset Parameters
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLBindParameter: SQLFreeStmt SQL_RESET_PARAMS clears bindings",
                 "[odbc-api][bindparameter][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Bind and execute successfully
  SQLINTEGER param_value = 42;
  SQLLEN indicator = 0;
  ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Clear parameter bindings
  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  // Attempt to execute without rebinding - should fail with 07002 (COUNT field incorrect)
  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "07002", stmt_handle(), SQL_HANDLE_STMT);

  // Rebind and execute successfully again
  ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
}
