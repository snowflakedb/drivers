#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

// Dedicated SQLGetDiagField suite (SNOW-3235408): header fields (RecNumber 0)
// and record fields, plus edge cases.

namespace {
SQLRETURN provoke_stmt_error(SQLHSTMT stmt) {
  return SQLExecDirect(stmt, sqlchar("SELECT * FROM snow_3235408_field_table_does_not_exist"), SQL_NTS);
}

// Reads a character diagnostic field whole. A fixed buffer would truncate the
// message text (the driver appends the full error trace), so size the buffer
// from a length probe.
std::string diag_str(SQLHSTMT stmt, SQLSMALLINT rec, SQLSMALLINT field) {
  SQLSMALLINT len = 0;
  SQLRETURN probe = SQLGetDiagField(SQL_HANDLE_STMT, stmt, rec, field, nullptr, 0, &len);
  if (probe != SQL_SUCCESS && probe != SQL_SUCCESS_WITH_INFO) {
    return "<err>";
  }
  std::vector<SQLCHAR> buf(static_cast<size_t>(len) + 1, 0);
  SQLRETURN ret =
      SQLGetDiagField(SQL_HANDLE_STMT, stmt, rec, field, buf.data(), static_cast<SQLSMALLINT>(buf.size()), &len);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    return "<err>";
  }
  return std::string(reinterpret_cast<char*>(buf.data()));
}
}  // namespace

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_NUMBER is 0 after success and >=1 after error",
                 "[odbc-api][getdiagfield][diagnostics]") {
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS) == SQL_SUCCESS);
  SQLINTEGER count = -1;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_NUMBER, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(count == 0);

  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_NUMBER, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(count >= 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_RETURNCODE reflects the last call's return code",
                 "[odbc-api][getdiagfield][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  SQLRETURN rc = 0;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_RETURNCODE, &rc, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rc == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQLSTATE / NATIVE / MESSAGE match SQLGetDiagRec",
                 "[odbc-api][getdiagfield][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  SQLCHAR rec_state[6] = {};
  SQLINTEGER rec_native = 0;
  // Read the record message whole (it can exceed a fixed buffer) so it can be
  // compared byte-for-byte against SQLGetDiagField's SQL_DIAG_MESSAGE_TEXT.
  SQLSMALLINT rec_msg_len = 0;
  REQUIRE(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, rec_state, &rec_native, nullptr, 0, &rec_msg_len) !=
          SQL_ERROR);
  std::vector<SQLCHAR> rec_msg(static_cast<size_t>(rec_msg_len) + 1, 0);
  REQUIRE(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, rec_state, &rec_native, rec_msg.data(),
                        static_cast<SQLSMALLINT>(rec_msg.size()), &rec_msg_len) == SQL_SUCCESS);

  CHECK(diag_str(stmt_handle(), 1, SQL_DIAG_SQLSTATE) == std::string(reinterpret_cast<char*>(rec_state)));
  CHECK(diag_str(stmt_handle(), 1, SQL_DIAG_MESSAGE_TEXT) == std::string(reinterpret_cast<char*>(rec_msg.data())));

  SQLINTEGER field_native = 0;
  REQUIRE(SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 1, SQL_DIAG_NATIVE, &field_native, 0, nullptr) ==
          SQL_SUCCESS);
  CHECK(field_native == rec_native);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetDiagField: SQL_DIAG_CLASS_ORIGIN / SUBCLASS_ORIGIN are ODBC-defined values",
                 "[odbc-api][getdiagfield][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  std::string class_origin = diag_str(stmt_handle(), 1, SQL_DIAG_CLASS_ORIGIN);
  std::string subclass_origin = diag_str(stmt_handle(), 1, SQL_DIAG_SUBCLASS_ORIGIN);
  // Per the ODBC spec these are either "ISO 9075" or "ODBC 3.0".
  CHECK((class_origin == "ISO 9075" || class_origin == "ODBC 3.0"));
  CHECK((subclass_origin == "ISO 9075" || subclass_origin == "ODBC 3.0"));
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_SERVER_NAME (BD#114)",
                 "[odbc-api][getdiagfield][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  std::string server = diag_str(stmt_handle(), 1, SQL_DIAG_SERVER_NAME);
  // BD#114: the new driver populates SQL_DIAG_SERVER_NAME from the connection; the old
  // driver leaves it empty.
  NEW_DRIVER_ONLY("BD#114") { CHECK(!server.empty()); }
  OLD_DRIVER_ONLY("BD#114") { CHECK(server.empty()); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_ROW_COUNT returns rows affected after DML",
                 "[odbc-api][getdiagfield][diagnostics]") {
  // BD#116: iODBC's wide SQLGetDiagFieldW does not surface statement header fields. The driver
  // writes the correct value and returns SQL_SUCCESS regardless of entry point, but iODBC's wide
  // path returns SQL_ERROR without delivering it (the old driver additionally aborts inside its
  // own DiagManager: a pthread_mutex assertion in Simba::ODBC::DiagManager::SQLGetDiagFieldW).
  // The failure is at the DM boundary, so skip both drivers under iODBC; unixODBC covers it.
  SKIP_IODBC("BD#116 - iODBC wide SQLGetDiagFieldW does not surface SQL_DIAG_ROW_COUNT (both drivers)");
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("CREATE OR REPLACE TEMPORARY TABLE snow_3235408_dml (id INT)"),
                        SQL_NTS) == SQL_SUCCESS);
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO snow_3235408_dml VALUES (1), (2), (3)"), SQL_NTS) ==
          SQL_SUCCESS);

  SQLLEN row_count = -1;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_ROW_COUNT, &row_count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(row_count == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_DYNAMIC_FUNCTION_CODE after a SELECT (BD#115)",
                 "[odbc-api][getdiagfield][diagnostics]") {
  // BD#116: iODBC's wide SQLGetDiagFieldW does not surface this statement header field. The old
  // driver returns a non-success code and the new driver's SQL_SUCCESS never reaches the caller,
  // so the read fails at the DM boundary for both drivers. Skip under iODBC; unixODBC (and the
  // reference lane) still cover BD#115's per-driver DYNAMIC_FUNCTION_CODE classification below.
  SKIP_IODBC("BD#116 - iODBC wide SQLGetDiagFieldW does not surface SQL_DIAG_DYNAMIC_FUNCTION_CODE (both drivers)");
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS) == SQL_SUCCESS);

  SQLINTEGER code = -12345;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_DYNAMIC_FUNCTION_CODE, &code, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  // BD#115: the new driver classifies the statement (SELECT -> SQL_DIAG_SELECT_CURSOR == 85);
  // the old driver returns 0 (SQL_DIAG_UNKNOWN_STATEMENT).
  NEW_DRIVER_ONLY("BD#115") { CHECK(code == SQL_DIAG_SELECT_CURSOR); }
  OLD_DRIVER_ONLY("BD#115") { CHECK(code == 0); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: invalid DiagIdentifier returns SQL_ERROR",
                 "[odbc-api][getdiagfield][diagnostics][error]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  SQLINTEGER out = 0;
  // 9999 is within SQLSMALLINT range but is not a defined diagnostic field identifier.
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 1, 9999, &out, 0, nullptr);
  CHECK(ret == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: negative RecNumber returns SQL_ERROR",
                 "[odbc-api][getdiagfield][diagnostics][error]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  SQLCHAR buf[64] = {};
  SQLSMALLINT len = 0;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), -1, SQL_DIAG_SQLSTATE, buf, sizeof(buf), &len);
  CHECK(ret == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagField: SQL_DIAG_CURSOR_ROW_COUNT reflects result-set size",
                 "[odbc-api][getdiagfield][diagnostics]") {
  // BD#116: iODBC's wide SQLGetDiagFieldW does not surface statement header fields. The driver
  // writes the correct value and returns SQL_SUCCESS regardless of entry point, but iODBC's wide
  // path returns SQL_ERROR without delivering it (the old driver additionally aborts inside its
  // own DiagManager: a pthread_mutex assertion in Simba::ODBC::DiagManager::SQLGetDiagFieldW).
  // The failure is at the DM boundary, so skip both drivers under iODBC; unixODBC covers it.
  SKIP_IODBC("BD#116 - iODBC wide SQLGetDiagFieldW does not surface SQL_DIAG_CURSOR_ROW_COUNT (both drivers)");
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 7))"), SQL_NTS) ==
          SQL_SUCCESS);
  SQLLEN cursor_rows = -1;
  SQLRETURN ret =
      SQLGetDiagField(SQL_HANDLE_STMT, stmt_handle(), 0, SQL_DIAG_CURSOR_ROW_COUNT, &cursor_rows, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(cursor_rows == 7);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLGetDiagField: statement-only header fields return SQL_ERROR on a connection handle",
                 "[odbc-api][getdiagfield][diagnostics][error]") {
  SQLINTEGER int_out = 0;
  SQLLEN len_out = 0;
  // SQL_DIAG_NUMBER is valid on a connection handle on both drivers.
  CHECK(SQLGetDiagField(SQL_HANDLE_DBC, dbc_handle(), 0, SQL_DIAG_NUMBER, &int_out, 0, nullptr) == SQL_SUCCESS);
  // Statement-only header fields are undefined on a connection handle; both drivers reject them
  // with SQL_ERROR (spec-correct), the same as on a descriptor handle (below).
  //
  // SQL_DIAG_DYNAMIC_FUNCTION_CODE is deliberately NOT asserted here: the Driver Manager
  // (unixODBC) services that header field from its own connection-handle cache and returns
  // SQL_SUCCESS without forwarding the call to the driver, so neither driver's guard is
  // reachable through a DM. ROW_COUNT and CURSOR_ROW_COUNT are not DM-serviced this way and
  // do reach the driver.
  CHECK(SQLGetDiagField(SQL_HANDLE_DBC, dbc_handle(), 0, SQL_DIAG_ROW_COUNT, &len_out, 0, nullptr) == SQL_ERROR);
  CHECK(SQLGetDiagField(SQL_HANDLE_DBC, dbc_handle(), 0, SQL_DIAG_CURSOR_ROW_COUNT, &len_out, 0, nullptr) == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetDiagField: statement-only header fields return SQL_ERROR on a descriptor handle",
                 "[odbc-api][getdiagfield][descriptor][diagnostics][error]") {
  SQLHDESC ard = SQL_NULL_HDESC;
  REQUIRE(SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr) == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);

  SQLINTEGER int_out = 0;
  SQLLEN len_out = 0;
  // SQL_DIAG_NUMBER is valid on a descriptor handle...
  CHECK(SQLGetDiagField(SQL_HANDLE_DESC, ard, 0, SQL_DIAG_NUMBER, &int_out, 0, nullptr) == SQL_SUCCESS);
  // ...but the statement-only header fields are not.
  CHECK(SQLGetDiagField(SQL_HANDLE_DESC, ard, 0, SQL_DIAG_ROW_COUNT, &len_out, 0, nullptr) == SQL_ERROR);
  CHECK(SQLGetDiagField(SQL_HANDLE_DESC, ard, 0, SQL_DIAG_CURSOR_ROW_COUNT, &len_out, 0, nullptr) == SQL_ERROR);
}

TEST_CASE("SQLGetDiagField: SQL_INVALID_HANDLE for a null handle", "[odbc-api][getdiagfield][diagnostics][error]") {
  SQLINTEGER int_out = 0;
  SQLSMALLINT len = 0;
  SQLRETURN ret = SQLGetDiagField(SQL_HANDLE_STMT, SQL_NULL_HSTMT, 0, SQL_DIAG_NUMBER, &int_out, 0, &len);
  CHECK(ret == SQL_INVALID_HANDLE);
}
