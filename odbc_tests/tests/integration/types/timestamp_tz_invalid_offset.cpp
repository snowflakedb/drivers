#include <sql.h>
#include <sqlext.h>

#include <algorithm>
#include <iterator>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

TEST_CASE("should reject an invalid TIMESTAMP_TZ biased offset", "[datatype][timestamp_tz][conversion][char]") {
  // Given the server returns a TIMESTAMP_TZ Arrow value whose biased offset is below zero
  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_any.json");
  wm.add_mapping_file("query/timestamp_tz_invalid_biased_offset.json");
  wm.add_mapping_file("session/logout_success.json");

  ensure_driver_installed();
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  std::string connection_string = get_wiremock_connection_string(wm);
  connection_string += "ODBC_QUERY_RESULT_FORMAT=ARROW;";
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(connection_string.c_str()), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  auto stmt = dbc.createStatementHandle();
  std::string query = "SELECT invalid_timestamp_tz";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When the row is fetched and converted to character data
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR value[64];
  std::fill(std::begin(value), std::end(value), static_cast<SQLCHAR>(0xFF));
  SQLLEN indicator = -1;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, value, sizeof(value), &indicator);

  const std::string sqlstate = get_sqlstate(stmt);
  INFO("SQLGetData ret=" << ret << " sqlstate=\"" << sqlstate << "\"");
  CAPTURE(ret, sqlstate);
  CHECK(ret == SQL_ERROR);
  CHECK(sqlstate == "HY000");
}
