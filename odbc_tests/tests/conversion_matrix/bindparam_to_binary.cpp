#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo BINARY_SQL_TYPES[] = {
  {SQL_BINARY,         "SQL_BINARY"},
  {SQL_VARBINARY,      "SQL_VARBINARY"},
  {SQL_LONGVARBINARY,  "SQL_LONGVARBINARY"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> BINARY SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][binary]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_binary (val BINARY)");
  ResultWriter report(get_report_path("bindparam_to_binary"));

  // When each C type is bound to each BINARY SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : BINARY_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_binary VALUES (?)", ct, st, report);
    }
  }
}
