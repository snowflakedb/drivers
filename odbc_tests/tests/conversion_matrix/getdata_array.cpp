#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: ARRAY -> all C types via SQLGetData", "[conversion_matrix][getdata][array]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_array"));

  // When an ARRAY containing nested objects is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT PARSE_JSON('[{\"id\": 1, \"name\": \"Alice\"}, {\"id\": 2, \"name\": \"Bob\"}]')",
                "ARRAY", ct, report);
  }
}
