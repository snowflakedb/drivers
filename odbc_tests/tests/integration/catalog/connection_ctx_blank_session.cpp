#include <picojson.h>
#include <sql.h>
#include <sqlext.h>

#include <cctype>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

namespace {

std::string ascii_lower(std::string value) {
  for (char& ch : value) {
    ch = static_cast<char>(std::tolower(static_cast<unsigned char>(ch)));
  }
  return value;
}

bool contains_ci(const std::string& haystack, const std::string& needle) {
  return ascii_lower(haystack).find(ascii_lower(needle)) != std::string::npos;
}

std::vector<std::string> query_sql_texts(const std::vector<picojson::value>& requests) {
  std::vector<std::string> sqls;
  for (const auto& req : requests) {
    REQUIRE(req.is<picojson::object>());
    const auto& req_obj = req.get<picojson::object>();
    const auto body_it = req_obj.find("body");
    REQUIRE(body_it != req_obj.end());
    REQUIRE(body_it->second.is<std::string>());

    picojson::value body_json;
    const std::string err = picojson::parse(body_json, body_it->second.get<std::string>());
    REQUIRE(err.empty());
    REQUIRE(body_json.is<picojson::object>());
    const auto& body = body_json.get<picojson::object>();
    const auto sql_it = body.find("sqlText");
    REQUIRE(sql_it != body.end());
    REQUIRE(sql_it->second.is<std::string>());
    sqls.push_back(sql_it->second.get<std::string>());
  }
  return sqls;
}

std::vector<std::string> connect_and_list_table(WiremockClient& wm, const char* login_mapping) {
  wm.add_mapping_file(login_mapping);
  wm.add_mapping_file("query/show_objects_empty.json");
  wm.add_mapping_file("session/logout_success.json");

  ensure_driver_installed();
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  const std::string connection_string = get_wiremock_connection_string(wm);
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(connection_string.c_str()), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  auto stmt = dbc.createStatementHandle();
  ret = SQLTables(stmt.getHandle(), nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS, nullptr, 0);
  REQUIRE_ODBC(ret, stmt);
  // The 3.x driver sends the SHOW from the first fetch.
  while (true) {
    const SQLRETURN fetch_ret = SQLFetch(stmt.getHandle());
    if (fetch_ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE((fetch_ret == SQL_SUCCESS || fetch_ret == SQL_SUCCESS_WITH_INFO));
  }
  const auto sqls = query_sql_texts(wm.find_requests("/queries/v1/query-request.*"));
  for (const auto& sql : sqls) {
    UNSCOPED_INFO(sql);
  }
  std::vector<std::string> objects;
  for (const auto& sql : sqls) {
    if (contains_ci(sql, "objects")) {
      objects.push_back(sql);
    }
  }
  return objects;
}

}  // namespace

TEST_CASE("SQLTables: blank session schema with connection context lists the session database",
          "[integration][catalog][tables]") {
  WiremockClient wm;
  const auto objects = connect_and_list_table(wm, "auth/login_success_ctx_blank_schema.json");
  REQUIRE(objects.size() == 1);
  CHECK(contains_ci(objects[0], "in database \"MOCKDB\""));
  CHECK_FALSE(contains_ci(objects[0], "in schema"));
}

TEST_CASE("SQLTables: blank session database with connection context lists the account",
          "[integration][catalog][tables]") {
  WiremockClient wm;
  const auto objects = connect_and_list_table(wm, "auth/login_success_ctx_blank_database.json");
  REQUIRE(objects.size() == 1);
  CHECK(contains_ci(objects[0], "in account"));
  CHECK_FALSE(contains_ci(objects[0], "in schema"));
  CHECK_FALSE(contains_ci(objects[0], "in database"));
}
