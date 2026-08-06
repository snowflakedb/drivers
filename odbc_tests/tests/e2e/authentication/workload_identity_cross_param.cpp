#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

// Cross-driver behavior for a WIF-only param supplied under a non-WIF authenticator.
//
// BD#108 (odbc_tests/BehaviorDifferences.yaml): the legacy snowflake-odbc driver registers
// WORKLOAD_IDENTITY_PROVIDER / WORKLOAD_IDENTITY_ENTRA_RESOURCE / WORKLOAD_IDENTITY_IMPERSONATION_PATH
// as accepted keys but reads them only inside the authenticator==workload_identity branch, so a
// non-WIF authenticator accepts the attribute and silently ignores it. The universal driver's
// sf_core::validate_settings rejects the combination (ConflictingParameters, Error), matching legacy
// snowflake-connector-python's long-standing rejection (ProgrammingError errno 251017).
TEST_CASE("should reject a WIF param under a non-WIF authenticator on the new driver but ignore it on the old driver",
          "[workload_identity_cross_param]") {
  // Given the default connection string (AUTHENTICATOR=SNOWFLAKE_JWT) plus a WIF-only param.
  auto conn_str = get_connection_string() + "WORKLOAD_IDENTITY_PROVIDER=AWS;";

  // When Trying to Connect
  // Then the new driver rejects the cross-param combination while the legacy driver silently
  // ignores the WIF param and connects (BD#108).
  NEW_DRIVER_ONLY("BD#108") {
    // sf_core rejects the cross-param combination before login. SQLDriverConnect returns SQL_ERROR
    // (asserted by require_connection_failed); the diagnostic carries a config-attribute SQLSTATE
    // and a message naming the offending WIF param. The message check anchors on both the param
    // name AND the distinguishing rejection phrase sf_core's ConflictingParameters check emits, so
    // an unrelated error that merely mentions the param name in passing can't satisfy this.
    auto records = require_connection_failed(conn_str);
    REQUIRE(records.size() >= 1);
    CHECK(records[0].sqlState == "01S00");
    CHECK_THAT(records[0].messageText, ContainsSubstring("workload_identity_provider"));
    CHECK_THAT(records[0].messageText, ContainsSubstring("was not set to WORKLOAD_IDENTITY"));
  }

  OLD_DRIVER_ONLY("BD#108") {
    // The legacy driver silently ignores the WIF param and the connection succeeds.
    Connection conn(conn_str);
    auto stmt = conn.execute_fetch("SELECT 1");
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
  }
}
