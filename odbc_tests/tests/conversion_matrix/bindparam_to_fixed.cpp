#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo FIXED_SQL_TYPES[] = {
  {SQL_INTEGER,  "SQL_INTEGER"},
  {SQL_SMALLINT, "SQL_SMALLINT"},
  {SQL_BIGINT,   "SQL_BIGINT"},
  {SQL_TINYINT,  "SQL_TINYINT"},
  {SQL_DECIMAL,  "SQL_DECIMAL"},
  {SQL_NUMERIC,  "SQL_NUMERIC"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> FIXED SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][fixed]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_fixed (val NUMBER)");
  ResultWriter report(get_report_path("bindparam_to_fixed"));

  // When each C type is bound to each FIXED SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : FIXED_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_fixed VALUES (?)", ct, st, report);
    }
  }
}
