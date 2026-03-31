#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo INTERVAL_SQL_TYPES[] = {
  {SQL_INTERVAL_YEAR,             "SQL_INTERVAL_YEAR"},
  {SQL_INTERVAL_MONTH,            "SQL_INTERVAL_MONTH"},
  {SQL_INTERVAL_DAY,              "SQL_INTERVAL_DAY"},
  {SQL_INTERVAL_HOUR,             "SQL_INTERVAL_HOUR"},
  {SQL_INTERVAL_MINUTE,           "SQL_INTERVAL_MINUTE"},
  {SQL_INTERVAL_SECOND,           "SQL_INTERVAL_SECOND"},
  {SQL_INTERVAL_YEAR_TO_MONTH,    "SQL_INTERVAL_YEAR_TO_MONTH"},
  {SQL_INTERVAL_DAY_TO_HOUR,      "SQL_INTERVAL_DAY_TO_HOUR"},
  {SQL_INTERVAL_DAY_TO_MINUTE,    "SQL_INTERVAL_DAY_TO_MINUTE"},
  {SQL_INTERVAL_DAY_TO_SECOND,    "SQL_INTERVAL_DAY_TO_SECOND"},
  {SQL_INTERVAL_HOUR_TO_MINUTE,   "SQL_INTERVAL_HOUR_TO_MINUTE"},
  {SQL_INTERVAL_HOUR_TO_SECOND,   "SQL_INTERVAL_HOUR_TO_SECOND"},
  {SQL_INTERVAL_MINUTE_TO_SECOND, "SQL_INTERVAL_MINUTE_TO_SECOND"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> INTERVAL SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][interval]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_interval (val VARCHAR)");
  ResultWriter report(get_report_path("bindparam_to_interval"));

  // When each C type is bound to each INTERVAL SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : INTERVAL_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_interval VALUES (?)", ct, st, report);
    }
  }
}
