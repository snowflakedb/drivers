#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: OBJECT -> all C types via SQLGetData", "[conversion_matrix][getdata][object]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_object"));

  // When a nested OBJECT with mixed types is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn,
                "SELECT OBJECT_CONSTRUCT("
                "'name', 'Alice'::VARIANT, "
                "'age', 30::VARIANT, "
                "'active', TRUE::VARIANT, "
                "'address', OBJECT_CONSTRUCT('city', 'NYC'::VARIANT, 'zip', '10001'::VARIANT)::VARIANT"
                ")",
                "OBJECT", ct, report);
  }
}
