#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: TIMESTAMP_TZ -> all C types via SQLGetData",
          "[conversion_matrix][getdata][timestamp_tz]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_timestamp_tz"));

  // When TIMESTAMP_TZ value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT '2024-01-15 12:30:45 -0800'::TIMESTAMP_TZ", "TIMESTAMP_TZ", ct, report);
  }
}
