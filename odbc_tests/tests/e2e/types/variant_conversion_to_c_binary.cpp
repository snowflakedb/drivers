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

TEST_CASE("VARIANT to SQL_C_BINARY", "[variant][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"b\":2}')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);

  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("VARIANT to SQL_C_BINARY array value", "[variant][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('[1,2,3]')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);

  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
}

TEST_CASE("VARIANT to SQL_C_BINARY empty object", "[variant][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{}')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);

  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
  CHECK(json.get<picojson::object>().empty());
}

TEST_CASE("VARIANT to SQL_C_BINARY nested value", "[variant][conversion][c_binary]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":{\"b\":[1,2]}}')");
  SQLCHAR buffer[512] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);

  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("VARIANT NULL to SQL_C_BINARY", "[variant][conversion][c_binary][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
