#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <random>
#include <sstream>
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
  auto dir = base / ("odbc_log_e2e_" + std::to_string(dist(gen)));
  fs::create_directories(dir);
  return dir;
}

static std::string read_all_files_in(const fs::path& dir) {
  std::ostringstream combined;
  if (!fs::exists(dir)) return {};
  for (const auto& entry : fs::directory_iterator(dir)) {
    if (entry.is_regular_file()) {
      std::ifstream in(entry.path(), std::ios::binary);
      combined << in.rdbuf();
    }
  }
  return combined.str();
}

TEST_CASE("should create log file when sf.odbc.ini configures file logging", "[logging]") {
  SKIP_OLD_DRIVER("BD#000", "Old driver does not support file logging setup via sf.odbc.ini");
  // Given a temp directory for logs and an sf.odbc.ini pointing there
  auto log_dir = create_temp_log_dir();
  auto ini_path = log_dir / "sf.odbc.ini";

  {
    std::ofstream ini(ini_path);
    ini << "LogLevel=DEBUG\n";
    ini << "LogPath=" << log_dir.string() << "\n";
    ini << "LogFile=odbc_e2e_test.log\n";
  }

  EnvOverride env_override("SF_ODBC_INI", ini_path.string());

  // When a connection is established and a query is executed
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 1");
  auto value = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(value == "1");

  // Then the log directory should contain a log file with connection-related output
  auto log_contents = read_all_files_in(log_dir);
  CHECK(!log_contents.empty());
  CHECK(log_contents.find("connect_with_params") != std::string::npos);

  // Cleanup
  std::error_code ec;
  fs::remove_all(log_dir, ec);
}
