// ODBC E2E: SQL_C_GUID bound via SQLBindParameter to SQL_VARCHAR.
//
// Snowflake has no native GUID column type, so per ODBC Appendix D
// ("Converting Data from C to SQL Data Types"), `SQL_C_GUID` →
// `SQL_VARCHAR/SQL_CHAR/SQL_WCHAR` is the canonical text route. The
// driver formats the 16-byte GUID as the standard 8-4-4-4-12 hex
// literal in upper-case (`Data1` and `Data2`/`Data3` are little-endian
// integer fields, `Data4` is a fixed byte sequence) — see
// `varchar.rs::WriteODBCType for SnowflakeVarchar` for the format
// string. These tests round-trip through Snowflake to confirm the on-
// wire text matches what the driver promises.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_guid_and_execute(StatementHandleWrapper& stmt, SQLGUID& val, SQLLEN& ind) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_GUID, SQL_VARCHAR, 36, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_GUID → VARCHAR
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_GUID to SQL_VARCHAR", "[c_guid][conversion][sql_string]") {
  // Given a VARCHAR column wide enough for the canonical 36-char form
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(40))");

  // When a canonical GUID is bound and inserted. The byte layout is
  // chosen so each section is visually distinct in the formatted output.
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLGUID val = {0x01234567, 0x89AB, 0xCDEF, {0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10}};
  SQLLEN ind = sizeof(val);
  bind_guid_and_execute(stmt, val, ind);

  // Then the formatted literal is the canonical 8-4-4-4-12 upper-case
  // hex form
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "01234567-89AB-CDEF-FEDC-BA9876543210");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind nil SQL_C_GUID to SQL_VARCHAR", "[c_guid][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(40))");

  // When the all-zero "nil" GUID is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLGUID val = {0, 0, 0, {0, 0, 0, 0, 0, 0, 0, 0}};
  SQLLEN ind = sizeof(val);
  bind_guid_and_execute(stmt, val, ind);

  // Then every section is rendered with full-width zero padding rather
  // than collapsed (the format string uses `:08X / :04X / :02X` widths)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "00000000-0000-0000-0000-000000000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind max SQL_C_GUID to SQL_VARCHAR", "[c_guid][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(40))");

  // When an all-`F` GUID is bound and inserted (verifies the formatter
  // doesn't accidentally sign-extend or mask any field)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLGUID val = {0xFFFFFFFF, 0xFFFF, 0xFFFF, {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}};
  SQLLEN ind = sizeof(val);
  bind_guid_and_execute(stmt, val, ind);

  // Then every section is rendered as the maximum hex value
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_GUID with NULL indicator to SQL_VARCHAR",
                 "[c_guid][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(40))");

  // When SQL_C_GUID is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN bind_ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_GUID, SQL_VARCHAR, 36, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(bind_ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
