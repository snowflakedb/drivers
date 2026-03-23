#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "cross_thread_cancel.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

namespace {
constexpr int kMaxPollIterations = 300;
}  // namespace

// ============================================================================
// SQLCancel - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on idle statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after query execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
  UNIX_ONLY {
    OLD_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

      ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
      REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
    }
    NEW_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);
    }
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after fetch", "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  UNIX_ONLY {
    OLD_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

      ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
      REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
    }
    NEW_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_NO_DATA);
    }
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on prepared but not executed statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

// ============================================================================
// SQLCancel - Statement State After Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: After cancel on executed prepared statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
  UNIX_ONLY {
    OLD_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

      ret = SQLExecute(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
    }
    NEW_DRIVER_ONLY("BD#30") {
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);
    }
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Statement recoverable via SQLFreeStmt SQL_CLOSE after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: SQLCloseCursor after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  WINDOWS_ONLY {
    ret = SQLCloseCursor(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
  UNIX_ONLY {
    OLD_DRIVER_ONLY("BD#30") {
      ret = SQLCloseCursor(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
    }
    NEW_DRIVER_ONLY("BD#30") {
      ret = SQLCloseCursor(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);
    }
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after error recovery and re-execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table_xyz_999"), SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on never-executed statement then use and free",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLHSTMT fresh_stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Verify the handle is still usable after cancel
  ret = SQLExecDirect(fresh_stmt, sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(fresh_stmt, 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Multiple cancels on idle statement",
                 "[odbc-api][cancel][terminating_statement]") {
  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancel(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 99);
}

// ============================================================================
// SQLCancel - Interaction with Bindings
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves bound columns after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves bound parameters after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  param = 88;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 88);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Parameter bindings preserved after cancel with open cursor",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 55);

  // Cancel while cursor is open (no-op on new driver, closes cursor on old)
  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // Re-execute with updated parameter value — bindings should be intact
  param = 123;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 123);
}

// ============================================================================
// SQLCancel - Data-at-Execution
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancels data-at-execution operation",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 77"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Re-execute immediately after canceling data-at-execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Verify re-execution works directly without an intervening SQLFreeStmt
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLCancel - Error Cases
// ============================================================================

TEST_CASE("SQLCancel: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cancel][terminating_statement][error]") {
  const SQLRETURN ret = SQLCancel(SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQLCancel - State Coverage
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after all rows fetched",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after DDL execution",
                 "[odbc-api][cancel][terminating_statement]") {
  auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE OR REPLACE TABLE cancel_test_tmp (id INT)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on statement in Error state",
                 "[odbc-api][cancel][terminating_statement]") {
  auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table"), SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLCancel - Cursor Preservation (ODBC 3.5 spec-compliant no-op)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cursor remains usable after cancel on multi-row result",
                 "[odbc-api][cancel][terminating_statement]") {
  NEW_DRIVER_ONLY("BD#30") {
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT column1 FROM VALUES (1),(2),(3) ORDER BY 1"), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER val = 0;

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(val == 1);

    ret = SQLCancel(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(val == 2);

    ret = SQLCancel(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(val == 3);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#30") {
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT column1 FROM VALUES (1),(2),(3) ORDER BY 1"), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER val = 0;

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(val == 1);

    ret = SQLCancel(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    WINDOWS_ONLY {
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);
      ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
      REQUIRE(ret == SQL_SUCCESS);
      REQUIRE(val == 2);
    }
    UNIX_ONLY {
      ret = SQLFetch(stmt_handle());
      REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
    }
  }
}

// ============================================================================
// SQLCancel - Isolation
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCancel: Does not affect other statements on same connection",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt2_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt2_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 2);
}

// ============================================================================
// SQLCancel - Attribute Preservation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves statement attributes after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLULEN max_length = 1024;
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, reinterpret_cast<SQLPOINTER>(max_length), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN retrieved_max_length = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, &retrieved_max_length, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(retrieved_max_length == max_length);

  SQLULEN cursor_type = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);
}

// ============================================================================
// SQLCancel - Diagnostic Behavior
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Does not post diagnostic records on no-op cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancel(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR sql_state[6] = {};
  SQLINTEGER native_error = 0;
  SQLCHAR message[256] = {};
  SQLSMALLINT msg_len = 0;
  ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, sql_state, &native_error, message, sizeof(message), &msg_len);
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLCancel - Async Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Async cancel interrupts execution with HY008",
                 "[odbc-api][cancel][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#30", "Reference driver async cancel does not interrupt in-progress operations");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Use a long TIMELIMIT so the query cannot complete before the cancel.
  // If the poll returns before 30s, it must be because the cancel worked.
  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt_handle());
  REQUIRE(cancel_ret == SQL_SUCCESS);

  int polls = 0;
  SQLRETURN poll_ret = SQL_STILL_EXECUTING;
  while (poll_ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    poll_ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  }
  REQUIRE(poll_ret != SQL_STILL_EXECUTING);

  REQUIRE_EXPECTED_ERROR(poll_ret, "HY008", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Async cancel clears diagnostics and posts its own",
                 "[odbc-api][cancel][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#30", "Reference driver async cancel does not interrupt in-progress operations");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt_handle());
  REQUIRE(cancel_ret == SQL_SUCCESS);

  int polls = 0;
  SQLRETURN poll_ret = SQL_STILL_EXECUTING;
  while (poll_ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    poll_ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  }
  REQUIRE(poll_ret != SQL_STILL_EXECUTING);

  REQUIRE_EXPECTED_ERROR(poll_ret, "HY008", stmt_handle(), SQL_HANDLE_STMT);

  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(!records.empty());
  REQUIRE(records.size() == 1);
  REQUIRE(records[0].sqlState == "HY008");

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// The ODBC spec allows function completion despite the cancel instruction.
// This case is non-deterministic as we cannot distinguish "cancel was a no-op"
// from "cancel tried but the query finished first".

// ============================================================================
// SQLCancel - Cross-Thread Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cross-thread cancel interrupts execution with HY008",
                 "[odbc-api][cancel][terminating_statement][cross_thread]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = stmt_handle();
  CrossThreadCancel ctx;
  // 5-second delay lets the query reach the server before cancel fires.
  // Without it, SQLCancel can arrive before SQLExecDirect has sent the query,
  // leaving nothing to cancel and causing the query to run to completion.
  ctx.run(stmt, "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 60))", std::chrono::seconds(5));

  REQUIRE(ctx.cancel_result == SQL_SUCCESS);

  SQLRETURN exec_ret = ctx.exec_result.load();
  REQUIRE_EXPECTED_ERROR(exec_ret, "HY008", stmt, SQL_HANDLE_STMT);

  // Per ODBC spec, cross-thread cancel does NOT clear the diagnostic
  // records of the canceled function, and does NOT post its own.
  // The HY008 from the canceled SQLExecDirect should be the only record.
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt);
  REQUIRE(records.size() == 1);
  REQUIRE(records[0].sqlState == "HY008");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: HY018 when server declines cancel request",
                 "[odbc-api][cancel][terminating_statement][cross_thread]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#30", "Snowflake server does not decline cancel requests");

  const SQLHSTMT stmt = stmt_handle();
  CrossThreadCancel ctx;
  ctx.run(stmt, "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 10))", std::chrono::seconds(0));

  REQUIRE_EXPECTED_ERROR(ctx.cancel_result, "HY018", stmt, SQL_HANDLE_STMT);
}

// The ODBC spec describes a race where both SQLCancel and the original
// function return SQL_SUCCESS. In that case the Driver Manager assumes the
// cursor is closed by the cancel, so the application cannot use the cursor.
// This requires SQLCancel to arrive after SQLExecDirect enters the driver but
// before the query reaches the server. This is non-deterministic and untestable.

// ============================================================================
// SQLCancel - Connection-Level Async Conflict (HY010)
// ============================================================================
//
// Per ODBC spec, SQLCancel returns HY010 if a connection-level async function
// is still executing on the parent connection. Testing this requires enabling
// SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE on the connection handle.

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: HY010 on connection-level async conflict",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // NOTE: The reference driver reports SQL_ASYNC_DBC_CAPABLE via SQLGetInfo
  // but rejects enabling it with SQL_ERROR.
  const SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE,
                                          reinterpret_cast<SQLPOINTER>(SQL_ASYNC_DBC_ENABLE_ON), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", dbc_handle(), SQL_HANDLE_DBC);
}
