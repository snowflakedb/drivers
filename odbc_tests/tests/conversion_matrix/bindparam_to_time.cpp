#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo TIME_SQL_TYPES[] = {
  {SQL_TIME,      "SQL_TIME"},
  {SQL_TYPE_TIME, "SQL_TYPE_TIME"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> TIME SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][time]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_time (val TIME)");
  ResultWriter report(get_report_path("bindparam_to_time"));

  // When each C type is bound to each TIME SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : TIME_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_time VALUES (?)", ct, st, report);
    }
  }
}
