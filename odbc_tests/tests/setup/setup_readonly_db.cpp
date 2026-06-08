#include "sql_script_runner.hpp"
#include "utils.hpp"

TEST_CASE("Setup readonly metadata test database", "[setup]") {
  const auto sql_path = test_utils::repo_root() / "scripts" / "odbc" / "setup_readonly_metadata_db.sql";
  sql_script_runner::run_sql_script(sql_path);
}
