#include <picojson.h>
#include <sql.h>
#include <sqlext.h>

#include <array>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// TODO(SNOW-4010544): this only exercises the sync SQLExecDirect retry path.
// `execute()` (SQLExecute after SQLPrepare) and `execute_dae()` apply the same
// retry but have no equivalent WireMock coverage. SQL_ATTR_ASYNC_ENABLE is a
// production gap as well: `run_cancellable` returns Spawned before the
// stage-binding error exists, so the retry match never runs and
// `complete_async_poll` surfaces the failure with no JSON fallback.
TEST_CASE("should retry with inline JSON bindings when the SYSTEM$BIND stage is disabled",
          "[query][bindings][stage_binding_retry]") {
  SKIP_OLD_DRIVER("BD#78", "Stage-binding-disabled retry is universal-driver-only");

  // Given CLIENT_STAGE_ARRAY_BINDING_THRESHOLD is 1, so a 2-row array bind selects
  // stage (CSV) binding, and CREATE TEMPORARY STAGE SYSTEM$BIND fails
  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_low_stage_binding_threshold.json");
  wm.add_mapping_file("query/create_stage_binding_disabled.json");
  wm.add_mapping_file("query/insert_success_after_stage_binding_retry.json");
  wm.add_mapping_file("session/logout_success.json");

  ensure_driver_installed();
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  std::string connection_string = get_wiremock_connection_string(wm);
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(connection_string.c_str()), SQL_NTS, nullptr, 0,
                                   nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);

  auto stmt = dbc.createStatementHandle();
  std::array<SQLINTEGER, 2> values = {1, 2};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(2)),
                       0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, values.data(),
                         sizeof(SQLINTEGER), nullptr);
  REQUIRE_ODBC(ret, stmt);

  // When the statement is executed
  std::string query = "INSERT INTO t (id) VALUES (?)";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);

  // Then it succeeds, having transparently retried with inline JSON bindings
  const std::string sqlstate = get_sqlstate(stmt);
  INFO("SQLExecDirect ret=" << ret << " sqlstate=\"" << sqlstate << "\"");
  CAPTURE(ret, sqlstate);
  CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  auto requests = wm.find_requests("/queries/v1/query-request.*");
  REQUIRE(requests.size() == 2);

  const auto parse_request_body = [](const picojson::value& req) -> picojson::object {
    const auto& req_obj = req.get<picojson::object>();
    auto body_it = req_obj.find("body");
    REQUIRE(body_it != req_obj.end());
    REQUIRE(body_it->second.is<std::string>());

    picojson::value body_json;
    std::string err = picojson::parse(body_json, body_it->second.get<std::string>());
    REQUIRE(err.empty());
    REQUIRE(body_json.is<picojson::object>());
    return body_json.get<picojson::object>();
  };

  const picojson::object create_stage_body = parse_request_body(requests[0]);
  const std::string create_stage_sql = create_stage_body.at("sqlText").get<std::string>();
  CHECK(create_stage_sql.find("CREATE TEMPORARY STAGE") != std::string::npos);
  CHECK(create_stage_body.find("bindings") == create_stage_body.end());

  const picojson::object insert_body = parse_request_body(requests[1]);
  const std::string insert_sql = insert_body.at("sqlText").get<std::string>();
  CHECK(insert_sql.find("INSERT INTO t") != std::string::npos);
  CHECK(insert_body.find("bindStage") == insert_body.end());

  auto bindings_it = insert_body.find("bindings");
  REQUIRE(bindings_it != insert_body.end());
  REQUIRE(bindings_it->second.is<picojson::object>());
  const auto& bindings_obj = bindings_it->second.get<picojson::object>();
  REQUIRE_FALSE(bindings_obj.empty());

  auto param1_it = bindings_obj.find("1");
  REQUIRE(param1_it != bindings_obj.end());
  REQUIRE(param1_it->second.is<picojson::object>());
  const auto& param1 = param1_it->second.get<picojson::object>();
  CHECK(param1.at("type").get<std::string>() == "FIXED");
  REQUIRE(param1.at("value").is<picojson::array>());
  const auto& bound_ids = param1.at("value").get<picojson::array>();
  REQUIRE(bound_ids.size() == 2);
  CHECK(bound_ids[0].get<std::string>() == "1");
  CHECK(bound_ids[1].get<std::string>() == "2");
}
