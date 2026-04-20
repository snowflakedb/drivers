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

TEST_CASE("ARRAY to SQL_C_BINARY", "[array][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as array with correct element count
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 3);
}

TEST_CASE("ARRAY to SQL_C_BINARY empty", "[array][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An empty ARRAY is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT()");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as empty array
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().empty());
}

TEST_CASE("ARRAY to SQL_C_BINARY nested", "[array][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A nested ARRAY is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1,2), ARRAY_CONSTRUCT(3,4))");
  SQLCHAR buffer[512] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as nested array
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 2);
}

TEST_CASE("ARRAY to SQL_C_BINARY buffer too small", "[array][conversion][c_binary][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched into a buffer smaller than the JSON representation
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");
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

TEST_CASE("ARRAY to SQL_C_BINARY exact buffer fit", "[array][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched into an exact-size buffer
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");

  SQLCHAR probe[256] = {};
  SQLLEN full_len = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, probe, sizeof(probe), &full_len);
  REQUIRE(ret == SQL_SUCCESS);

  // Then The indicator equals the buffer size used and the data is valid JSON
  CHECK(full_len > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(probe), static_cast<size_t>(full_len)));
  CHECK(json.is<picojson::array>());
}

TEST_CASE("ARRAY NULL to SQL_C_BINARY", "[array][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL ARRAY value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
