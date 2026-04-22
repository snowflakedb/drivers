#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_BINARY — raw bytes
// ============================================================================

TEST_CASE("should convert binary to SQL_C_BINARY returning raw bytes", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

  // Then SQL_C_BINARY should return raw bytes [0x48, 0x65, 0x6C, 0x6C, 0x6F]
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 5);
  REQUIRE(buffer[0] == 0x48);
  REQUIRE(buffer[1] == 0x65);
  REQUIRE(buffer[2] == 0x6C);
  REQUIRE(buffer[3] == 0x6C);
  REQUIRE(buffer[4] == 0x6F);
}

// ============================================================================
// SQL_C_DEFAULT — same as SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert binary to SQL_C_DEFAULT returning raw bytes", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'CAFE'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'CAFE'::BINARY");

  // Then SQL_C_DEFAULT should return same result as SQL_C_BINARY
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 2);
  REQUIRE(buffer[0] == 0xCA);
  REQUIRE(buffer[1] == 0xFE);
}

// ============================================================================
// SQLBindCol with SQL_C_BINARY
// ============================================================================

TEST_CASE("should retrieve binary via SQLBindCol with SQL_C_BINARY", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  const auto stmt = conn.createStatement();

  // When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_BINARY
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT X'ABCDEF'::BINARY"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound buffer should contain raw bytes [0xAB, 0xCD, 0xEF]
  REQUIRE(indicator == 3);
  REQUIRE(buffer[0] == 0xAB);
  REQUIRE(buffer[1] == 0xCD);
  REQUIRE(buffer[2] == 0xEF);
}

// ============================================================================
// Empty binary conversion
// ============================================================================

TEST_CASE("should convert empty binary to SQL_C_BINARY returning zero-length data",
          "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X''::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X''::BINARY");

  // Then SQL_C_BINARY should return indicator 0
  SQLCHAR buffer[16] = {0xFF};
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 0);
  REQUIRE(buffer[0] == 0xFF);
}

// ============================================================================
// VARBINARY synonym — SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert VARBINARY to SQL_C_BINARY same as BINARY", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'CAFE'::VARBINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'CAFE'::VARBINARY");

  // Then SQL_C_BINARY should return raw bytes [0xCA, 0xFE]
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 2);
  REQUIRE(buffer[0] == 0xCA);
  REQUIRE(buffer[1] == 0xFE);
}

// ============================================================================
// Unsupported target type
// ============================================================================

TEST_CASE("should return error when converting binary to unsupported C type",
          "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'ABCDEF'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'ABCDEF'::BINARY");

  // Then SQLGetData with SQL_C_FLOAT should return SQL_ERROR
  SQLFLOAT value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_FLOAT, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_ERROR);
  REQUIRE(get_sqlstate(stmt) == "07006");
}

// ============================================================================
// Column metadata via SQLColAttribute
// TODO: Blocked on general SQLColAttribute support for SQL_DESC_OCTET_LENGTH and
// SQL_DESC_PRECISION (not binary-specific; these attributes have per-type semantics).
// See untagged scenario in binary_conversion_to_c_binary.feature.
// ============================================================================

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL binary with SQL_C_BINARY", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::BINARY");

  // Then SQL_C_BINARY should return SQL_NULL_DATA indicator
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == SQL_NULL_DATA);
}

// ============================================================================
// Chunked SQLGetData for large binary
// ============================================================================

TEST_CASE("should retrieve large binary in chunks via SQLGetData", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a binary value larger than the buffer is executed
  const auto stmt = conn.execute_fetch("SELECT X'0102030405060708'::BINARY");

  SQLCHAR buffer[4] = {};
  SQLLEN indicator = 0;

  // Then First SQLGetData call should return SQL_SUCCESS_WITH_INFO with partial data
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 8);
  REQUIRE(buffer[0] == 0x01);
  REQUIRE(buffer[1] == 0x02);
  REQUIRE(buffer[2] == 0x03);
  REQUIRE(buffer[3] == 0x04);

  // And Second SQLGetData call should return SQL_SUCCESS with remaining data
  memset(buffer, 0, sizeof(buffer));
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == 4);
  REQUIRE(buffer[0] == 0x05);
  REQUIRE(buffer[1] == 0x06);
  REQUIRE(buffer[2] == 0x07);
  REQUIRE(buffer[3] == 0x08);
}

// ============================================================================
// Exact-fit buffer for SQL_C_BINARY
// ============================================================================

TEST_CASE("should succeed with exact-fit buffer for SQL_C_BINARY", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'ABCDEF'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'ABCDEF'::BINARY");

  // Then SQL_C_BINARY with buffer exactly matching data length should return SQL_SUCCESS
  SQLCHAR buffer[3] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, 3, &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == 3);
  REQUIRE(buffer[0] == 0xAB);
  REQUIRE(buffer[1] == 0xCD);
  REQUIRE(buffer[2] == 0xEF);
}

// ============================================================================
// 3-chunk retrieval for SQL_C_BINARY
// ============================================================================

TEST_CASE("should retrieve binary in three chunks via SQLGetData", "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a 9-byte binary value is executed
  const auto stmt = conn.execute_fetch("SELECT X'010203040506070809'::BINARY");

  SQLCHAR buffer[3] = {};
  SQLLEN indicator = 0;

  // Then First SQLGetData call should return first 3 bytes with 01004
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, 3, &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 9);
  REQUIRE(buffer[0] == 0x01);
  REQUIRE(buffer[1] == 0x02);
  REQUIRE(buffer[2] == 0x03);

  // And Second SQLGetData call should return next 3 bytes with 01004
  memset(buffer, 0, sizeof(buffer));
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, 3, &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 6);
  REQUIRE(buffer[0] == 0x04);
  REQUIRE(buffer[1] == 0x05);
  REQUIRE(buffer[2] == 0x06);

  // And Third SQLGetData call should return final 3 bytes with SQL_SUCCESS
  memset(buffer, 0, sizeof(buffer));
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, 3, &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == 3);
  REQUIRE(buffer[0] == 0x07);
  REQUIRE(buffer[1] == 0x08);
  REQUIRE(buffer[2] == 0x09);
}

// ============================================================================
// Zero-length buffer for SQL_C_BINARY — length-only query
// ============================================================================

TEST_CASE("should report full length with zero-length buffer for SQL_C_BINARY",
          "[datatype][binary][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'ABCDEF'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'ABCDEF'::BINARY");

  // Then SQLGetData with BufferLength=0 should return 01004 with indicator reporting full data length
  SQLCHAR dummy = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, &dummy, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 3);
}
