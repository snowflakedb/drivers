#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo TIMESTAMP_SQL_TYPES[] = {
  {SQL_TIMESTAMP,      "SQL_TIMESTAMP"},
  {SQL_TYPE_TIMESTAMP, "SQL_TYPE_TIMESTAMP"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> TIMESTAMP SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][timestamp]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_timestamp (val TIMESTAMP_NTZ)");
  ResultWriter report(get_report_path("bindparam_to_timestamp"));

  // When each C type is bound to each TIMESTAMP SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : TIMESTAMP_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_timestamp VALUES (?)", ct, st, report);
    }
  }
}
