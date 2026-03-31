#include "conversion_matrix_common.hpp"

TEST_CASE("conversion matrix: NUMBER(integer) -> all C types via SQLGetData", "[conversion_matrix][getdata][number]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_number_int"));

  // When integer NUMBER value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT 42::NUMBER(10,0)", "NUMBER_INT", ct, report);
  }
}

TEST_CASE("conversion matrix: NUMBER(decimal) -> all C types via SQLGetData", "[conversion_matrix][getdata][number]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_number_dec"));

  // When decimal NUMBER value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, "SELECT 3.14::NUMBER(10,2)", "NUMBER_DEC", ct, report);
  }
}
