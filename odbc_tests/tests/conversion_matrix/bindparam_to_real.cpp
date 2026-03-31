#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo REAL_SQL_TYPES[] = {
  {SQL_REAL,   "SQL_REAL"},
  {SQL_FLOAT,  "SQL_FLOAT"},
  {SQL_DOUBLE, "SQL_DOUBLE"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> REAL SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][real]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_real (val FLOAT)");
  ResultWriter report(get_report_path("bindparam_to_real"));

  // When each C type is bound to each REAL SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : REAL_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_real VALUES (?)", ct, st, report);
    }
  }
}
