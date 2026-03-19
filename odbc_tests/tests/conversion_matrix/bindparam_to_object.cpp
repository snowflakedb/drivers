#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo OBJECT_SQL_TYPES[] = {
  {SQL_VARCHAR, "SQL_VARCHAR"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> OBJECT column via SQLBindParameter",
          "[conversion_matrix][bindparam][object]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_object (val OBJECT)");
  ResultWriter report(get_report_path("bindparam_to_object"));

  // When each C type is bound to SQL_VARCHAR targeting an OBJECT column
  // Then results are recorded to CSV
  for (const auto& st : OBJECT_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_object SELECT PARSE_JSON(?)", ct, st, report);
    }
  }
}
