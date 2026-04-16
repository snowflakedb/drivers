#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

static picojson::value parse_json(const std::string& text) {
  picojson::value v;
  REQUIRE(picojson::parse(v, text).empty());
  return v;
}

TEST_CASE("OBJECT to SQL_C_BINARY", "[object][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as object
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("OBJECT to SQL_C_BINARY empty", "[object][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An empty OBJECT is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT()");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as empty object
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
  CHECK(json.get<picojson::object>().empty());
}

TEST_CASE("OBJECT to SQL_C_BINARY nested", "[object][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A nested OBJECT is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 42))");
  SQLCHAR buffer[512] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable with nested structure intact
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
  auto inner = json.get<picojson::object>().find("outer");
  REQUIRE(inner != json.get<picojson::object>().end());
  CHECK(inner->second.is<picojson::object>());
}

TEST_CASE("OBJECT to SQL_C_BINARY buffer too small", "[object][conversion][c_binary][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched into a buffer smaller than the JSON representation
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')");
  SQLCHAR buffer[4] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator > static_cast<SQLLEN>(sizeof(buffer)));
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("OBJECT to SQL_C_BINARY exact buffer fit", "[object][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched into an exact-size buffer
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')");

  SQLCHAR probe[256] = {};
  SQLLEN full_len = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, probe, sizeof(probe), &full_len);
  REQUIRE(ret == SQL_SUCCESS);

  // Then The indicator equals the buffer size used and the data is valid JSON
  CHECK(full_len > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(probe), static_cast<size_t>(full_len)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("OBJECT NULL to SQL_C_BINARY", "[object][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL OBJECT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
