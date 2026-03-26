#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: TIMESTAMP_NTZ -> all C types via SQLGetData", "[conversion_matrix][getdata][timestamp]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_timestamp"));

  // When TIMESTAMP_NTZ value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT '2024-01-15 12:30:45'::TIMESTAMP_NTZ", "TIMESTAMP_NTZ", ct, report);
  }
}
