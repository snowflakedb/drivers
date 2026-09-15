#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

namespace {

SQLRETURN driver_connect(ConnectionHandleWrapper& dbc, const std::string& conn_str) {
  return SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                          SQL_DRIVER_NOPROMPT);
}

const DiagRec* find_diag(const std::vector<DiagRec>& records, const std::string& sql_state) {
  for (const auto& rec : records) {
    if (rec.sqlState == sql_state) {
      return &rec;
    }
  }
  return nullptr;
}

}  // namespace

TEST_CASE("should report PUT_MAXRETRIES deprecation only in the new driver", "[integration][connection][BD#141]") {
  WiremockClient wm(WiremockClient::Mode::Server);
  wm.add_mapping_file("auth/login_success_any.json");
  const std::string conn_str = get_wiremock_connection_string(wm) + "PUT_MAXRETRIES=3;";

  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  const SQLRETURN ret = driver_connect(dbc, conn_str);
  const auto records = get_diag_rec(dbc);
  NEW_DRIVER_ONLY("BD#141") {
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
    // iODBC's DM drops the driver's SQLDriverConnect SUCCESS_WITH_INFO record
    // (BD#61); the 01000 text is asserted on unixODBC / Windows.
    NON_IODBC {
      const DiagRec* warning = find_diag(records, "01000");
      REQUIRE(warning != nullptr);
      CHECK_THAT(warning->messageText,
                 ContainsSubstring("Parameter 'PUT_MAXRETRIES' is deprecated, use 'PUT_GET_MAX_ATTEMPTS' instead"));
    }
  }
  OLD_DRIVER_ONLY("BD#141") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(records.empty());
  }
}

TEST_CASE("should report GET_MAXRETRIES deprecation only in the new driver", "[integration][connection][BD#141]") {
  WiremockClient wm(WiremockClient::Mode::Server);
  wm.add_mapping_file("auth/login_success_any.json");
  const std::string conn_str = get_wiremock_connection_string(wm) + "GET_MAXRETRIES=4;";

  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  const SQLRETURN ret = driver_connect(dbc, conn_str);
  const auto records = get_diag_rec(dbc);
  NEW_DRIVER_ONLY("BD#141") {
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
    // iODBC's DM drops the driver's SQLDriverConnect SUCCESS_WITH_INFO record
    // (BD#61); the 01000 text is asserted on unixODBC / Windows.
    NON_IODBC {
      const DiagRec* warning = find_diag(records, "01000");
      REQUIRE(warning != nullptr);
      CHECK_THAT(warning->messageText,
                 ContainsSubstring("Parameter 'GET_MAXRETRIES' is deprecated, use 'PUT_GET_MAX_ATTEMPTS' instead"));
    }
  }
  OLD_DRIVER_ONLY("BD#141") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(records.empty());
  }
}
