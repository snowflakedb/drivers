// ODBC E2E: SQL_C_BINARY (and the SQL_C_DEFAULT alias) bound via
// SQLBindParameter to SQL_BINARY / SQL_VARBINARY / SQL_LONGVARBINARY. The
// bound buffer is forwarded to Snowflake verbatim (hex-encoded on the wire),
// and the round-trip via SQLGetData(SQL_C_BINARY) must return the exact same
// bytes.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types", section
// "Binary"), SQL_C_BINARY is the canonical source for binary SQL targets and
// SQL_C_DEFAULT resolves to it for these targets.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <array>
#include <cstdint>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr std::array<SQLCHAR, 4> kPayload = {0xDE, 0xAD, 0xBE, 0xEF};

void check_binary_round_trip(Connection& conn, const std::string& column_type, SQLSMALLINT param_type,
                             SQLSMALLINT value_type) {
  conn.execute("CREATE TEMPORARY TABLE t (col " + column_type + ")");

  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  std::array<SQLCHAR, 4> payload = kPayload;
  SQLLEN ind = static_cast<SQLLEN>(payload.size());
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, value_type, param_type, 0, 0, payload.data(),
                                static_cast<SQLLEN>(payload.size()), &ind),
               stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  std::array<SQLCHAR, 16> buffer = {};
  SQLLEN out_ind = 0;
  REQUIRE_ODBC(SQLGetData(fetch_stmt.getHandle(), 1, SQL_C_BINARY, buffer.data(), buffer.size(), &out_ind), fetch_stmt);
  CHECK(out_ind == 4);
  CHECK(buffer[0] == 0xDE);
  CHECK(buffer[1] == 0xAD);
  CHECK(buffer[2] == 0xBE);
  CHECK(buffer[3] == 0xEF);
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY raw bytes to SQL_BINARY and read back",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary BINARY column exists
  // When the four-byte payload 0xDEADBEEF is bound as SQL_C_BINARY -> SQL_BINARY and executed
  // Then the round-trip via SQLGetData(SQL_C_BINARY) returns the same four bytes
  check_binary_round_trip(conn, "BINARY", SQL_BINARY, SQL_C_BINARY);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY raw bytes to SQL_VARBINARY and read back",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary VARBINARY column exists
  // When the four-byte payload 0xDEADBEEF is bound as SQL_C_BINARY -> SQL_VARBINARY and executed
  // Then the round-trip via SQLGetData(SQL_C_BINARY) returns the same four bytes
  check_binary_round_trip(conn, "VARBINARY", SQL_VARBINARY, SQL_C_BINARY);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY raw bytes to SQL_LONGVARBINARY and read back",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary BINARY column exists (Snowflake exposes BINARY as the
  // single variable-length binary class for which SQL_LONGVARBINARY is the
  // ODBC alias)
  // When the four-byte payload 0xDEADBEEF is bound as SQL_C_BINARY -> SQL_LONGVARBINARY and executed
  // Then the round-trip via SQLGetData(SQL_C_BINARY) returns the same four bytes
  check_binary_round_trip(conn, "BINARY", SQL_LONGVARBINARY, SQL_C_BINARY);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT raw bytes to SQL_BINARY and read back",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary BINARY column exists
  // When the four-byte payload 0xDEADBEEF is bound as SQL_C_DEFAULT -> SQL_BINARY and executed
  // Then the bind resolves to SQL_C_BINARY per ODBC Appendix D and the round-trip returns the same four bytes
  check_binary_round_trip(conn, "BINARY", SQL_BINARY, SQL_C_DEFAULT);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY zero-length payload to SQL_BINARY and read back empty",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary BINARY column exists
  conn.execute("CREATE TEMPORARY TABLE t (col BINARY)");

  // When SQL_C_BINARY is bound with indicator=0 (zero-length payload) and executed
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLCHAR payload[1] = {0};
  SQLLEN ind = 0;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, 0, 0, payload,
                                sizeof(payload), &ind),
               stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the column round-trips as a zero-byte BINARY value
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQLCHAR buffer[16] = {};
  SQLLEN out_ind = 0;
  REQUIRE_ODBC(SQLGetData(fetch_stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &out_ind), fetch_stmt);
  CHECK(out_ind == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject oversize SQL_C_BINARY payload against bounded SQL_BINARY column",
                 "[c_binary][conversion][sql_binary]") {
  // Given a temporary BINARY(4) column exists (the driver does not enforce
  // the column size at SQLExecute; the server-side validation is the only
  // gate)
  conn.execute("CREATE TEMPORARY TABLE t (col BINARY(4))");

  // When an 8-byte SQL_C_BINARY payload is bound and executed against the 4-byte column
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  std::array<SQLCHAR, 8> payload = {0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE};
  SQLLEN ind = static_cast<SQLLEN>(payload.size());
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, 0, 0, payload.data(),
                                static_cast<SQLLEN>(payload.size()), &ind),
               stmt);

  // Then the server rejects the insert; this test pins the current behavior
  // so any future change (silent truncation, success, different SQLSTATE)
  // shows up as a failure here rather than a silent matrix flip
  SQLRETURN ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError());
}
