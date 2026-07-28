#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"

namespace fs = std::filesystem;

static fs::path create_temp_log_dir() {
  auto base = fs::temp_directory_path();
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<uint64_t> dist;
  auto dir = base / ("odbc_troubleshooting_e2e_" + std::to_string(dist(gen)));
  fs::create_directories(dir);
  return dir;
}

TEST_CASE("should create troubleshooting log file when enabled via environment variable", "[logging]") {
  SKIP_OLD_DRIVER("", "Old driver does not support SNOWFLAKE_TROUBLESHOOTING_ENABLED env var");

  // Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true" and
  //   SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
  auto log_dir = create_temp_log_dir();
  EnvOverride ts_enabled("SNOWFLAKE_TROUBLESHOOTING_ENABLED", "true");
  EnvOverride ts_path("SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH", log_dir.string());

  // When a connection is established and a query is executed
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 1");
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(value == "1");

  // Then a troubleshooting log file exists in the configured directory
  auto log_file = log_dir / "sf_driver_troubleshooting.log";
  CHECK(fs::exists(log_file));

  // And the log file contains debug-level entries below the configured log level
  std::ifstream in(log_file, std::ios::binary);
  std::string contents((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
  CHECK(!contents.empty());
  CHECK(contents.find("Login successful, extracting session tokens") != std::string::npos);

  std::error_code ec;
  fs::remove_all(log_dir, ec);
  if (ec) {
    WARN("cleanup failed: " << ec.message());
  }
}
