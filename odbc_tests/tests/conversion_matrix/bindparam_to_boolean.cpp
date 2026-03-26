#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo BOOLEAN_SQL_TYPES[] = {
  {SQL_BIT, "SQL_BIT"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> BOOLEAN SQL type via SQLBindParameter",
          "[conversion_matrix][bindparam][boolean]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_boolean (val BOOLEAN)");
  ResultWriter report(get_report_path("bindparam_to_boolean"));

  // When each C type is bound to SQL_BIT and executed
  // Then results are recorded to CSV
  for (const auto& st : BOOLEAN_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_boolean VALUES (?)", ct, st, report);
    }
  }
}
