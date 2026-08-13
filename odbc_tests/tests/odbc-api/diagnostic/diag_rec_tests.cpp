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

// Dedicated SQLGetDiagRec suite (SNOW-3235408). These exercise the record and
// edge-case behavior directly instead of relying on the error-checking macros
// used throughout the other suites.

namespace {
// Provoke a deterministic server-side error so a diagnostic record exists on
// the statement handle.
SQLRETURN provoke_stmt_error(SQLHSTMT stmt) {
  return SQLExecDirect(stmt, sqlchar("SELECT * FROM snow_3235408_table_does_not_exist"), SQL_NTS);
}
}  // namespace

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: valid record after a query error",
                 "[odbc-api][getdiagrec][diagnostics]") {
  // Given a statement that failed with a server-side error
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  // When the first diagnostic record is retrieved. The message can exceed any
  // fixed buffer (the driver appends the full error trace), so size the buffer
  // from a length probe to read it whole and expect an untruncated SQL_SUCCESS.
  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLSMALLINT full_len = 0;
  REQUIRE(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, nullptr, 0, &full_len) != SQL_ERROR);
  REQUIRE(full_len > 0);

  std::vector<SQLCHAR> msg(static_cast<size_t>(full_len) + 1, 0);
  SQLSMALLINT msg_len = 0;
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, msg.data(),
                                static_cast<SQLSMALLINT>(msg.size()), &msg_len);

  // Then SQLSTATE, native error and message text are populated
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string(reinterpret_cast<char*>(state)).size() == 5);
  CHECK(msg_len == full_len);
  CHECK(std::string(reinterpret_cast<char*>(msg.data())).size() == static_cast<size_t>(msg_len));

  // BD#110: for a server (data source-originated) error the two drivers diverge in how they prefix the
  // diagnostic message text. The new driver prepends the ODBC §16.2.16 [vendor][ODBC-component]
  // identifier plus a [data-source] segment, yielding the 3-part
  // "[Snowflake][Snowflake ODBC Driver][Snowflake]". The old (SimbaEngine) driver does NOT prefix
  // server-originated messages at all: it passes the raw server text through, starting with
  // "SQL compilation error:". (The old driver's "[Snowflake][Support]" vendor prefix appears only on
  // driver-originated errors) The INFO capture surfaces the
  // actual message on any mismatch.
  std::string message(reinterpret_cast<char*>(msg.data()));
  INFO("diagnostic message: " << message);
  NEW_DRIVER_ONLY("BD#110") { CHECK(message.rfind("[Snowflake][Snowflake ODBC Driver][Snowflake]", 0) == 0); }
  OLD_DRIVER_ONLY("BD#110") { CHECK(message.rfind("SQL compilation error", 0) == 0); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: RecNumber 0 returns SQL_ERROR",
                 "[odbc-api][getdiagrec][diagnostics][error]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 0, state, &native, msg, sizeof(msg), &msg_len);
  CHECK(ret == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: RecNumber past the last record returns SQL_NO_DATA",
                 "[odbc-api][getdiagrec][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 999, state, &native, msg, sizeof(msg), &msg_len);
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: negative BufferLength returns SQL_ERROR",
                 "[odbc-api][getdiagrec][diagnostics][error]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  // A negative buffer length is invalid (HY090). Asserted on both drivers; if the
  // old driver diverges here, the reference lane will flag it as a real difference
  // to document as a BD.
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, msg, -1, &msg_len);
  CHECK(ret == SQL_ERROR);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: zero BufferLength reports full length via TextLengthPtr",
                 "[odbc-api][getdiagrec][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLSMALLINT msg_len = 0;
  // BufferLength 0 with a null message buffer: success-with-info, full length reported.
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, nullptr, 0, &msg_len);
  CHECK((ret == SQL_SUCCESS_WITH_INFO || ret == SQL_SUCCESS));
  CHECK(msg_len > 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetDiagRec: small buffer truncates with SQL_SUCCESS_WITH_INFO and full length",
                 "[odbc-api][getdiagrec][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  // First read the full length.
  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLSMALLINT full_len = 0;
  REQUIRE(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, nullptr, 0, &full_len) != SQL_ERROR);
  REQUIRE(full_len > 4);

  // Then read into a buffer too small to hold it.
  SQLCHAR small[5] = {};
  SQLSMALLINT reported = 0;
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, small, sizeof(small), &reported);
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(reported == full_len);              // full (untruncated) length reported
  CHECK(small[sizeof(small) - 1] == '\0');  // null-terminated within the buffer
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: null output pointers are individually tolerated",
                 "[odbc-api][getdiagrec][diagnostics]") {
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);

  // Every optional output pointer may be null without error.
  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  CHECK(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, nullptr, &native, msg, sizeof(msg), &msg_len) != SQL_ERROR);
  CHECK(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, nullptr, msg, sizeof(msg), &msg_len) != SQL_ERROR);
  CHECK(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, nullptr, 0, &msg_len) != SQL_ERROR);
  CHECK(SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, msg, sizeof(msg), nullptr) != SQL_ERROR);
}

TEST_CASE("SQLGetDiagRec: SQL_INVALID_HANDLE for a null handle", "[odbc-api][getdiagrec][diagnostics][error]") {
  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, SQL_NULL_HSTMT, 1, state, &native, msg, sizeof(msg), &msg_len);
  CHECK(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetDiagRec: diagnostics are cleared by the next call on the same handle",
                 "[odbc-api][getdiagrec][diagnostics]") {
  // Given a statement carrying a diagnostic record from a failed execute
  REQUIRE(provoke_stmt_error(stmt_handle()) == SQL_ERROR);
  SQLCHAR state[6] = {};
  SQLINTEGER native = 0;
  SQLCHAR msg[256] = {};
  SQLSMALLINT msg_len = 0;
  // This setup read only needs to confirm a record exists; the message may exceed
  // this buffer and truncate (SQL_SUCCESS_WITH_INFO), which is fine here.
  SQLRETURN setup = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, msg, sizeof(msg), &msg_len);
  REQUIRE((setup == SQL_SUCCESS || setup == SQL_SUCCESS_WITH_INFO));

  // When the next successful call runs on the same handle
  REQUIRE(SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS) == SQL_SUCCESS);

  // Then the previous diagnostic record is gone
  SQLRETURN ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, state, &native, msg, sizeof(msg), &msg_len);
  CHECK(ret == SQL_NO_DATA);
}
