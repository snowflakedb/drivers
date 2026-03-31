#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: BINARY -> all C types via SQLGetData", "[conversion_matrix][getdata][binary]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_binary"));

  // When BINARY value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT TO_BINARY('48656C6C6F','HEX')", "BINARY", ct, report);
  }
}
