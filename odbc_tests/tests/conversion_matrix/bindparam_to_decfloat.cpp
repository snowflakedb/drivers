#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo DECFLOAT_SQL_TYPES[] = {
  {SQL_VARCHAR, "SQL_VARCHAR"},
  {SQL_DOUBLE,  "SQL_DOUBLE"},
  {SQL_DECIMAL, "SQL_DECIMAL"},
  {SQL_NUMERIC, "SQL_NUMERIC"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> DECFLOAT column via SQLBindParameter",
          "[conversion_matrix][bindparam][decfloat]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_decfloat (val DECFLOAT)");
  ResultWriter report(get_report_path("bindparam_to_decfloat"));

  // When each C type is bound to each candidate SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : DECFLOAT_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_decfloat VALUES (?)", ct, st, report);
    }
  }
}
