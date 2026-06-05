// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL string types
// (SQL_CHAR, SQL_VARCHAR, SQL_LONGVARCHAR, SQL_WCHAR, SQL_WVARCHAR,
// SQL_WLONGVARCHAR).
//
// Per MS ODBC "Converting Data from C to SQL Data Types: Binary", binding a
// SQL_C_BINARY source to a character SQL target renders the raw bytes as their
// hexadecimal representation. The new driver implements this (lowercase hex).
// The legacy driver does not hex-encode: it forwards the raw bytes to
// Snowflake, so the outcome depends on the payload -- non-UTF-8 bytes are
// rejected with HTTP 400 while UTF-8-valid bytes are accepted as the raw
// string. That value-dependent divergence is documented as BD#34, so the
// hex-encoding assertions run on the new driver only.

#include <sql.h>
#include <sqlext.h>

#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

struct StringTarget {
  SQLSMALLINT sql_type;
  const char* name;
};

// clang-format off
const StringTarget STRING_TARGETS[] = {
    {SQL_CHAR,         "SQL_CHAR"},
    {SQL_VARCHAR,      "SQL_VARCHAR"},
    {SQL_LONGVARCHAR,  "SQL_LONGVARCHAR"},
    {SQL_WCHAR,        "SQL_WCHAR"},
    {SQL_WVARCHAR,     "SQL_WVARCHAR"},
    {SQL_WLONGVARCHAR, "SQL_WLONGVARCHAR"},
};
// clang-format on

// Binds `bytes` as SQL_C_BINARY to the given SQL string target, runs
// `SELECT ? AS val`, and asserts the lowercase-hex rendering. Hex-encoding is a
// new-driver behavior (BD#34): the legacy driver forwards the raw bytes
// instead, so its outcome varies with the input (non-UTF-8 payloads error with
// HTTP 400, UTF-8-valid payloads round-trip as the raw string) and it does not
// uniformly return SQL_ERROR. Callers therefore SKIP_OLD_DRIVER; the legacy
// raw-passthrough rejection is pinned separately by the canonical
// c_binary_to_varchar query test.
void check_binary_to_string(SQLSMALLINT sql_type, std::vector<unsigned char> bytes, const std::string& expected_hex) {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLLEN indicator = static_cast<SQLLEN>(bytes.size());
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 100, 0, bytes.data(),
                                   static_cast<SQLLEN>(bytes.size()), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == expected_hex);
}

}  // namespace

TEST_CASE("should hex-encode SQL_C_BINARY bound to every SQL string target", "[c_binary][conversion][sql_string]") {
  // Given Snowflake client is logged in
  SKIP_OLD_DRIVER("BD#34", "Legacy driver forwards raw bytes instead of hex-encoding SQL_C_BINARY -> string");
  for (const auto& target : STRING_TARGETS) {
    INFO("target = " << target.name);
    // When a 4-byte binary buffer is bound to the string target and selected
    std::vector<unsigned char> bytes = {0xDE, 0xAD, 0xBE, 0xEF};
    // Then the new driver renders the bytes as lowercase hex (legacy: BD#34)
    check_binary_to_string(target.sql_type, bytes, "deadbeef");
  }
}

TEST_CASE("should hex-encode SQL_C_BINARY containing embedded NUL and high bytes",
          "[c_binary][conversion][sql_string]") {
  // Given Snowflake client is logged in
  SKIP_OLD_DRIVER("BD#34", "Legacy driver forwards raw bytes instead of hex-encoding SQL_C_BINARY -> string");
  // When a binary buffer with an embedded 0x00 and 0xFF is bound to SQL_VARCHAR
  std::vector<unsigned char> bytes = {0x00, 0x01, 0xFF, 0x10};
  // Then every byte is rendered, including the NUL, as two hex digits
  check_binary_to_string(SQL_VARCHAR, bytes, "0001ff10");
}

TEST_CASE("should hex-encode a single SQL_C_BINARY byte to SQL_VARCHAR", "[c_binary][conversion][sql_string]") {
  // Given Snowflake client is logged in
  SKIP_OLD_DRIVER("BD#34", "Legacy driver forwards raw bytes instead of hex-encoding SQL_C_BINARY -> string");
  // When a single 0x00 byte is bound to SQL_VARCHAR and selected
  std::vector<unsigned char> bytes = {0x00};
  // Then it renders as "00" rather than an empty string
  check_binary_to_string(SQL_VARCHAR, bytes, "00");
}

TEST_CASE("should bind SQL_C_BINARY with NULL indicator to SQL_VARCHAR", "[c_binary][conversion][sql_string]") {
  // Given Snowflake client is logged in
  SKIP_OLD_DRIVER("BD#34", "Legacy driver forwards raw bytes instead of hex-encoding SQL_C_BINARY -> string");
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_C_BINARY is bound with SQL_NULL_DATA to SQL_VARCHAR and selected
  unsigned char param[] = {0xDE, 0xAD};
  SQLLEN indicator = SQL_NULL_DATA;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the selected value should be NULL
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
}
