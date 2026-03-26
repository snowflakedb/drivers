#include "conversion_matrix_common.hpp"

// clang-format off
static const SqlTypeInfo TEXT_SQL_TYPES[] = {
  {SQL_VARCHAR,        "SQL_VARCHAR"},
  {SQL_CHAR,           "SQL_CHAR"},
  {SQL_LONGVARCHAR,    "SQL_LONGVARCHAR"},
  {SQL_WCHAR,          "SQL_WCHAR"},
  {SQL_WVARCHAR,       "SQL_WVARCHAR"},
  {SQL_WLONGVARCHAR,   "SQL_WLONGVARCHAR"},
};
// clang-format on

TEST_CASE("conversion matrix: all C types -> TEXT SQL types via SQLBindParameter",
          "[conversion_matrix][bindparam][text]") {
  SKIP_UNLESS_PROGRESS_REPORT();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE OR REPLACE TABLE cm_text (val VARCHAR)");
  ResultWriter report(get_report_path("bindparam_to_text"));

  // When each C type is bound to each TEXT SQL type and executed
  // Then results are recorded to CSV
  for (const auto& st : TEXT_SQL_TYPES) {
    for (const auto& ct : ALL_C_TYPES) {
      try_bindparam(conn, "INSERT INTO cm_text VALUES (?)", ct, st, report);
    }
  }
}
