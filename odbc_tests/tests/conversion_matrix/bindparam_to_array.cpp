#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo ARRAY_SQL_TYPES[] = {
  {SQL_VARCHAR, "SQL_VARCHAR"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> ARRAY column via SQLBindParameter",
          "[conversion_matrix][bindparam][array]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_array (val ARRAY)");
  ResultWriter report(get_report_path("bindparam_to_array"));

  // When each C type is bound to SQL_VARCHAR targeting an ARRAY column
  // Then results are recorded to CSV
  for (const auto& st : ARRAY_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_array SELECT PARSE_JSON(?)", ct, st, report);
    }
  }
}
