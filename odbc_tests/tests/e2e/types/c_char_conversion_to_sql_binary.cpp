// ODBC E2E: SQL_C_CHAR / SQL_C_WCHAR bound via SQLBindParameter to
// SQL_BINARY / SQL_VARBINARY / SQL_LONGVARBINARY. Per ODBC Appendix D
// ("Converting Data from C to SQL Data Types", section "Binary"),
// character inputs are interpreted as ASCII hex literals: each pair of
// characters is decoded into one byte before forwarding to the server.
// Odd length or non-hex characters must surface as SQLSTATE 22018
// ("Invalid character value for cast specification").
//
// The old reference driver did not implement this decode and forwarded
// the raw string bytes hex-encoded, so "DEADBEEF" (8 ASCII bytes) would
// land on the server as 8 bytes (0x44 0x45 0x41 0x44 0x42 0x45 0x45 0x46)
// rather than the intended 4 bytes (0xDE 0xAD 0xBE 0xEF). See
// BehaviorDifferences.yaml #49.

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

void check_hex_literal_round_trip(Connection& conn, const std::string& column_type, SQLSMALLINT param_type,
                                  SQLCHAR* literal, SQLLEN literal_len_bytes) {
  conn.execute("CREATE TEMPORARY TABLE t (col " + column_type + ")");

  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, param_type, 0, 0, literal,
                                literal_len_bytes, &ind),
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

void check_wchar_hex_literal_round_trip(Connection& conn, const std::string& column_type, SQLSMALLINT param_type,
                                        SQLWCHAR* literal, SQLLEN literal_len_bytes) {
  conn.execute("CREATE TEMPORARY TABLE t (col " + column_type + ")");

  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, param_type, 0, 0, literal,
                                literal_len_bytes, &ind),
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

// ----------------------------------------------------------------------------
// SQL_C_CHAR positive paths - case variants across the three binary targets
// ----------------------------------------------------------------------------

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR uppercase hex literal to SQL_BINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  // When the uppercase hex literal "DEADBEEF" is bound as SQL_C_CHAR -> SQL_BINARY and executed
  // Then the driver decodes the literal and the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLCHAR literal[] = "DEADBEEF";
  check_hex_literal_round_trip(conn, "BINARY", SQL_BINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR lowercase hex literal to SQL_BINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  // When the lowercase hex literal "deadbeef" is bound as SQL_C_CHAR -> SQL_BINARY and executed
  // Then the driver decodes the literal case-insensitively and the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLCHAR literal[] = "deadbeef";
  check_hex_literal_round_trip(conn, "BINARY", SQL_BINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR mixed-case hex literal to SQL_BINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  // When the mixed-case hex literal "DeAdBeEf" is bound as SQL_C_CHAR -> SQL_BINARY and executed
  // Then the driver decodes the literal case-insensitively and the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLCHAR literal[] = "DeAdBeEf";
  check_hex_literal_round_trip(conn, "BINARY", SQL_BINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR hex literal to SQL_VARBINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary VARBINARY column exists
  // When the hex literal "DEADBEEF" is bound as SQL_C_CHAR -> SQL_VARBINARY and executed
  // Then the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLCHAR literal[] = "DEADBEEF";
  check_hex_literal_round_trip(conn, "VARBINARY", SQL_VARBINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR hex literal to SQL_LONGVARBINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists (Snowflake exposes BINARY as the
  // single variable-length binary class for which SQL_LONGVARBINARY is the
  // ODBC alias)
  // When the hex literal "DEADBEEF" is bound as SQL_C_CHAR -> SQL_LONGVARBINARY and executed
  // Then the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLCHAR literal[] = "DEADBEEF";
  check_hex_literal_round_trip(conn, "BINARY", SQL_LONGVARBINARY, literal, sizeof(literal));
}

// ----------------------------------------------------------------------------
// SQL_C_CHAR negative paths - malformed hex must surface as SQLSTATE 22018
// ----------------------------------------------------------------------------

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR odd-length hex literal bound to SQL_BINARY",
                 "[c_char][conversion][sql_binary][negative]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  conn.execute("CREATE TEMPORARY TABLE t (col BINARY)");

  // When the odd-length literal "DEADBEE" (7 chars) is bound as SQL_C_CHAR -> SQL_BINARY and executed
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLCHAR literal[] = "DEADBEE";
  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BINARY, 0, 0, literal,
                                sizeof(literal), &ind),
               stmt);

  // Then SQLExecute fails with SQLSTATE 22018 (Invalid character value for cast)
  SQLRETURN ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR non-hex characters bound to SQL_BINARY",
                 "[c_char][conversion][sql_binary][negative]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  conn.execute("CREATE TEMPORARY TABLE t (col BINARY)");

  // When the non-hex literal "GHIJKLMN" is bound as SQL_C_CHAR -> SQL_BINARY and executed
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLCHAR literal[] = "GHIJKLMN";
  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BINARY, 0, 0, literal,
                                sizeof(literal), &ind),
               stmt);

  // Then SQLExecute fails with SQLSTATE 22018 (Invalid character value for cast)
  SQLRETURN ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

// ----------------------------------------------------------------------------
// SQL_C_WCHAR positive paths - case variants across the three binary targets
// ----------------------------------------------------------------------------

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR uppercase hex literal to SQL_BINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  // When the UTF-16 uppercase hex literal L"DEADBEEF" is bound as SQL_C_WCHAR -> SQL_BINARY and executed
  // Then the driver transcodes to ASCII, decodes the literal, and the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLWCHAR literal[] = {'D', 'E', 'A', 'D', 'B', 'E', 'E', 'F', 0};
  check_wchar_hex_literal_round_trip(conn, "BINARY", SQL_BINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR lowercase hex literal to SQL_BINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  // When the UTF-16 lowercase hex literal L"deadbeef" is bound as SQL_C_WCHAR -> SQL_BINARY and executed
  // Then the driver transcodes and decodes case-insensitively, returning 4 bytes 0xDE 0xAD 0xBE 0xEF on round-trip
  SQLWCHAR literal[] = {'d', 'e', 'a', 'd', 'b', 'e', 'e', 'f', 0};
  check_wchar_hex_literal_round_trip(conn, "BINARY", SQL_BINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR hex literal to SQL_VARBINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary VARBINARY column exists
  // When the UTF-16 hex literal L"DEADBEEF" is bound as SQL_C_WCHAR -> SQL_VARBINARY and executed
  // Then the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLWCHAR literal[] = {'D', 'E', 'A', 'D', 'B', 'E', 'E', 'F', 0};
  check_wchar_hex_literal_round_trip(conn, "VARBINARY", SQL_VARBINARY, literal, sizeof(literal));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR hex literal to SQL_LONGVARBINARY and read back bytes",
                 "[c_char][conversion][sql_binary]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists (Snowflake exposes BINARY as the
  // single variable-length binary class for which SQL_LONGVARBINARY is the
  // ODBC alias)
  // When the UTF-16 hex literal L"DEADBEEF" is bound as SQL_C_WCHAR -> SQL_LONGVARBINARY and executed
  // Then the round-trip returns 4 bytes 0xDE 0xAD 0xBE 0xEF
  SQLWCHAR literal[] = {'D', 'E', 'A', 'D', 'B', 'E', 'E', 'F', 0};
  check_wchar_hex_literal_round_trip(conn, "BINARY", SQL_LONGVARBINARY, literal, sizeof(literal));
}

// ----------------------------------------------------------------------------
// SQL_C_WCHAR negative path - malformed hex must surface as SQLSTATE 22018
// ----------------------------------------------------------------------------

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_WCHAR odd-length hex literal bound to SQL_BINARY",
                 "[c_char][conversion][sql_binary][negative]") {
  SKIP_OLD_DRIVER("BD#49", "Old driver hex-encodes the ASCII string instead of decoding the hex literal");
  // Given a temporary BINARY column exists
  conn.execute("CREATE TEMPORARY TABLE t (col BINARY)");

  // When the odd-length UTF-16 literal L"DEADBEE" (7 chars) is bound as SQL_C_WCHAR -> SQL_BINARY and executed
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);

  SQLWCHAR literal[] = {'D', 'E', 'A', 'D', 'B', 'E', 'E', 0};
  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BINARY, 0, 0, literal,
                                sizeof(literal), &ind),
               stmt);

  // Then SQLExecute fails with SQLSTATE 22018 (Invalid character value for cast)
  SQLRETURN ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}
