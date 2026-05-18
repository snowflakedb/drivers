#include "conversion_matrix_common.hpp"

// Pick a VARCHAR literal that is well-formed for the target C type. For most
// targets a bare integer is fine, but composite SQL_C_INTERVAL_* types require
// a composite literal (e.g. '3-6' for YEAR_TO_MONTH) - probing them with '42'
// would report 22018 even when the driver implements Appendix D correctly.
static const char* varchar_probe_query_for(SQLSMALLINT c_type) {
  switch (c_type) {
    case SQL_C_INTERVAL_YEAR_TO_MONTH:
      return "SELECT '3-6'::VARCHAR";
    case SQL_C_INTERVAL_DAY_TO_HOUR:
      return "SELECT '5 10'::VARCHAR";
    case SQL_C_INTERVAL_DAY_TO_MINUTE:
      return "SELECT '3 14:30'::VARCHAR";
    case SQL_C_INTERVAL_DAY_TO_SECOND:
      return "SELECT '2 08:15:30'::VARCHAR";
    case SQL_C_INTERVAL_HOUR_TO_MINUTE:
      return "SELECT '10:45'::VARCHAR";
    case SQL_C_INTERVAL_HOUR_TO_SECOND:
      return "SELECT '12:30:45'::VARCHAR";
    case SQL_C_INTERVAL_MINUTE_TO_SECOND:
      return "SELECT '45:30'::VARCHAR";
    default:
      return "SELECT '42'::VARCHAR";
  }
}

TEST_CASE("conversion matrix: VARCHAR -> all C types via SQLGetData", "[conversion_matrix][getdata][varchar]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  ResultWriter report(get_report_path("getdata_varchar"));

  // When VARCHAR value is fetched as each C type
  // Then results are recorded to CSV
  for (const auto& ct : ALL_C_TYPES) {
    try_getdata(conn, varchar_probe_query_for(ct.c_type), "VARCHAR", ct, report);
  }
}
