#include <sql.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: returns HY092 on descriptor after SQLCancelHandle",
                 "[odbc-api][getdiagrec][descriptor][diagnostics]") {
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  ret = SQLCancelHandle(SQL_HANDLE_DESC, ard);
  WINDOWS_ONLY {
    REQUIRE(ret == SQL_ERROR);

    SQLCHAR state[6] = {};
    SQLINTEGER native = 0;
    SQLCHAR msg[256] = {};
    SQLSMALLINT msg_len = 0;
    ret = SQLGetDiagRec(SQL_HANDLE_DESC, ard, 1, state, &native, msg, sizeof(msg), &msg_len);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(std::string(reinterpret_cast<char*>(state), 5) == "HY092");
  }
  // unixODBC returns SQL_INVALID_HANDLE for unsupported handle types.
  UNIX_ONLY { REQUIRE(ret == SQL_INVALID_HANDLE); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: returns record count on descriptor after SQLCancelHandle",
                 "[odbc-api][getdiagfield][descriptor][diagnostics]") {
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  ret = SQLCancelHandle(SQL_HANDLE_DESC, ard);
  WINDOWS_ONLY {
    REQUIRE(ret == SQL_ERROR);

    SQLINTEGER num_records = 0;
    SQLSMALLINT str_len = 0;
    ret = SQLGetDiagField(SQL_HANDLE_DESC, ard, 0, SQL_DIAG_NUMBER, &num_records, 0, &str_len);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(num_records == 1);

    SQLCHAR state[6] = {};
    ret = SQLGetDiagField(SQL_HANDLE_DESC, ard, 1, SQL_DIAG_SQLSTATE, state, sizeof(state), &str_len);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(std::string(reinterpret_cast<char*>(state), 5) == "HY092");
  }
  // unixODBC returns SQL_INVALID_HANDLE for unsupported handle types.
  UNIX_ONLY { REQUIRE(ret == SQL_INVALID_HANDLE); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: SQL_NO_DATA on clean descriptor",
                 "[odbc-api][getdiagrec][descriptor][diagnostics]") {
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  ret = SQLGetDiagRec(SQL_HANDLE_DESC, ard, 1, state, &native, msg, sizeof(msg), &msg_len);
  CHECK(ret == SQL_NO_DATA);
}
