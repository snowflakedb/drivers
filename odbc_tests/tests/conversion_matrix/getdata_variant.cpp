#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: VARIANT -> all C types via SQLGetData", "[conversion_matrix][getdata][variant]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_variant"));

  // When VARIANT holding a nested object is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn,
                "SELECT PARSE_JSON('{\"name\": \"Alice\", \"age\": 30, \"active\": true, "
                "\"scores\": [95, 87, 92], \"address\": {\"city\": \"NYC\", \"zip\": \"10001\"}}')",
                "VARIANT", ct, report);
  }
}
