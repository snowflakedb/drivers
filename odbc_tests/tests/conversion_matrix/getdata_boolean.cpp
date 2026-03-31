#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: BOOLEAN -> all C types via SQLGetData", "[conversion_matrix][getdata][boolean]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_boolean"));

  // When BOOLEAN value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT TRUE::BOOLEAN", "BOOLEAN", ct, report);
  }
}
