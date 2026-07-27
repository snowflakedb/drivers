#include <sql.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: returns HY092 on descriptor after SQLCancelHandle",
                 "[odbc-api][getdiagrec][descriptor][diagnostics]") {
  // SQLCancelHandle is an ODBC 3.8 entry point that iODBC does not export, so
  // this scenario is unrunnable there.
  SKIP_IODBC("SQLCancelHandle (ODBC 3.8) is not exposed by iODBC");

  // Given a default statement with its ARD descriptor handle
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  // When SQLCancelHandle is called on the descriptor handle
  ret = SQLCancelHandle(SQL_HANDLE_DESC, ard);

  WINDOWS_ONLY {
    // Then the call returns SQL_ERROR and a diagnostic record with SQLSTATE HY092
    //   (invalid attribute identifier) is produced
    REQUIRE(ret == SQL_ERROR);

    SQLCHAR state[6] = {};
    SQLINTEGER native = 0;
    SQLCHAR msg[256] = {};
    SQLSMALLINT msg_len = 0;
    ret = SQLGetDiagRec(SQL_HANDLE_DESC, ard, 1, state, &native, msg, sizeof(msg), &msg_len);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(std::string(reinterpret_cast<char*>(state), 5) == "HY092");
  }
  UNIX_ONLY {
    // Then the call returns SQL_INVALID_HANDLE (no diagnostic record produced)
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: returns record count on descriptor after SQLCancelHandle",
                 "[odbc-api][getdiagfield][descriptor][diagnostics]") {
  // SQLCancelHandle is an ODBC 3.8 entry point that iODBC does not export, so
  // this scenario is unrunnable there.
  SKIP_IODBC("SQLCancelHandle (ODBC 3.8) is not exposed by iODBC");

  // Given a default statement with its ARD descriptor handle
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  // When SQLCancelHandle is called on the descriptor handle
  ret = SQLCancelHandle(SQL_HANDLE_DESC, ard);

  WINDOWS_ONLY {
    // Then the call returns SQL_ERROR and SQLGetDiagField sees one record with
    //   SQLSTATE HY092 (invalid attribute identifier)
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
  UNIX_ONLY {
    // Then the call returns SQL_INVALID_HANDLE for unsupported handle types
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: message text appends internal error trace by default",
                 "[odbc-api][getdiagrec][diagnostics]") {
  // BD#77: by default the new driver appends its internal ErrorTrace (file/line entries collected via
  // the `error_trace` derive) to the user-facing diagnostic message, under an `error trace:` header;
  // the gate `ErrorTraceEnabled` defaults to true. The old driver returns only the human-readable
  // message with no appended trace. Applications that match diagnostic text against fixed substrings, or
  // surface SQLGetDiagRec output verbatim, observe the extra trailer on the new driver.

  // When a statement that fails server-side compilation is executed (referenced object does not exist)
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM bd77_object_that_does_not_exist"), SQL_NTS);

  // Then the call reports an error and at least one diagnostic record is produced
  REQUIRE(ret == SQL_ERROR);
  const auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_FALSE(records.empty());

  // And the appended `error trace:` header is present on the new driver and absent on the old driver
  const bool has_error_trace = records[0].messageText.find("error trace:") != std::string::npos;
  NEW_DRIVER_ONLY("BD#77") { CHECK(has_error_trace); }
  OLD_DRIVER_ONLY("BD#77") { CHECK_FALSE(has_error_trace); }
}
