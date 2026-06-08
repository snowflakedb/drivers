#include "sql_script_runner.hpp"
#include "utils.hpp"

TEST_CASE("Setup datometry replay test database", "[setup]") {
  const auto sql_path = test_utils::repo_root() / "scripts" / "odbc" / "setup_datometry_replay.sql";
  sql_script_runner::run_sql_script(sql_path);
}
