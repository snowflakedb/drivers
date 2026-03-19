#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: DECFLOAT -> all C types via SQLGetData", "[conversion_matrix][getdata][decfloat]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_decfloat"));

  // When DECFLOAT value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT '1.23456789012345678901234567890'::DECFLOAT", "DECFLOAT", ct, report);
  }
}
