#include <sql.h>
#include <sqlext.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static void do_connect_cycle(const std::string& conn_str, const std::string& label) {
  INFO("Cycle: " << label);

  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDBC dbc = SQL_NULL_HDBC;
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc), OdbcMatchers::Succeeded());

  ret = SQLDisconnect(dbc);
  CHECK(ret == SQL_SUCCESS);
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  CHECK(ret == SQL_SUCCESS);
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  CHECK(ret == SQL_SUCCESS);
}

TEST_CASE("Environment re-init: alloc-free-alloc cycle does not crash", "[lifecycle][reinit]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif

  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_any.json");
  wm.add_mapping_file("session/logout_success.json");

  std::string conn_str = get_wiremock_connection_string(wm);

  // First full cycle — this always works
  do_connect_cycle(conn_str, "first (baseline)");

  // Second full cycle — triggers re-init of OdbcGlobals and LogManager.
  // Before the fix, this crashes or fails due to zombie global subscriber.
  do_connect_cycle(conn_str, "second (re-init)");
}

TEST_CASE("Environment re-init: three consecutive cycles", "[lifecycle][reinit]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif

  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_any.json");
  wm.add_mapping_file("session/logout_success.json");

  std::string conn_str = get_wiremock_connection_string(wm);

  for (int i = 1; i <= 3; ++i) {
    do_connect_cycle(conn_str, "cycle " + std::to_string(i));
  }
}
