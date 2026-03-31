#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo DATE_SQL_TYPES[] = {
  {SQL_DATE,      "SQL_DATE"},
  {SQL_TYPE_DATE, "SQL_TYPE_DATE"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> DATE SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][date]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_date (val DATE)");
  ResultWriter report(get_report_path("bindparam_to_date"));

  // When each C type is bound to each DATE SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : DATE_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_date VALUES (?)", ct, st, report);
    }
  }
}
