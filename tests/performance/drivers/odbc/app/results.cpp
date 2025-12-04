#include "results.h"

#include <sys/utsname.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <memory>
#include <sstream>

#include "put_execution.h"

// Forward declarations for private functions
std::string get_architecture();
std::string get_os_version();
std::unique_ptr<std::ofstream> open_csv_file(const std::string& filename);
void write_run_metadata_json(const std::string& driver_type, const std::string& driver_version,
                             const std::string& server_version, time_t timestamp,
                             const std::string& filename);

void write_csv_results(const std::vector<TestResult>& results, const std::string& filename) {
  auto csv = open_csv_file(filename);
  if (!csv) return;

  *csv << "timestamp,query_s,fetch_s\n";
  for (const auto& r : results) {
    *csv << r.timestamp << "," << std::fixed << std::setprecision(6) << r.query_time_s << ","
         << r.fetch_time_s << "\n";
  }
  csv->close();
}

void write_csv_results_put_get(const std::vector<PutGetResult>& results,
                               const std::string& filename) {
  auto csv = open_csv_file(filename);
  if (!csv) return;

  *csv << "timestamp,query_s\n";
  for (const auto& r : results) {
    *csv << r.timestamp << "," << std::fixed << std::setprecision(6) << r.query_time_s << "\n";
  }
  csv->close();
}

std::string generate_results_filename(const std::string& test_name, const std::string& driver_type,
                                      time_t timestamp) {
  std::filesystem::path results_dir = std::filesystem::path("/results");
  std::stringstream filename_ss;
  filename_ss << test_name << "_odbc_" << driver_type << "_" << timestamp << ".csv";
  return (results_dir / filename_ss.str()).string();
}

std::string generate_metadata_filename(const std::string& driver_type) {
  std::filesystem::path results_dir = std::filesystem::path("/results");
  std::stringstream metadata_filename_ss;
  metadata_filename_ss << "run_metadata_odbc_" << driver_type << ".json";
  return (results_dir / metadata_filename_ss.str()).string();
}

void finalize_test_execution(const std::string& results_file, const std::string& driver_type,
                             const std::string& driver_version, const std::string& server_version,
                             time_t timestamp) {
  std::string metadata_filename = generate_metadata_filename(driver_type);
  write_run_metadata_json(driver_type, driver_version, server_version, timestamp,
                          metadata_filename);
  std::cout << "\n✓ Complete → " << results_file << "\n";
}

std::string get_architecture() {
  struct utsname sys_info;
  if (uname(&sys_info) == 0) {
    std::string machine = sys_info.machine;

    if (machine == "amd64" || machine == "x86_64") {
      return "x86_64";
    } else if (machine == "aarch64") {
      return "arm64";
    }

    return machine;
  }
  return "unknown";
}

std::string get_os_version() {
  const char* os_info = std::getenv("OS_INFO");
  return os_info ? std::string(os_info) : "Linux";
}

std::unique_ptr<std::ofstream> open_csv_file(const std::string& filename) {
  std::filesystem::path filepath(filename);
  if (filepath.has_parent_path()) {
    std::filesystem::create_directories(filepath.parent_path());
  }

  auto csv = std::make_unique<std::ofstream>(filename);
  if (!csv->is_open()) {
    std::cerr << "ERROR: Failed to open file for writing: " << filename << "\n";
    return nullptr;
  }

  return csv;
}

void write_run_metadata_json(const std::string& driver_type, const std::string& driver_version,
                             const std::string& server_version, time_t timestamp,
                             const std::string& filename) {
  // Check if metadata file already exists
  std::ifstream check_file(filename);
  if (check_file.good()) {
    check_file.close();
    return;  // Metadata already exists, don't overwrite
  }

  // Detect architecture and OS inside container
  std::string architecture = get_architecture();
  std::string os = get_os_version();

  // Get Rust compiler version from environment (set during Docker build)
  const char* rust_version_env = std::getenv("RUST_VERSION");
  std::string build_rust_version = rust_version_env ? std::string(rust_version_env) : "unknown";

  std::ofstream json(filename);
  if (!json.is_open()) {
    std::cerr << "ERROR: Failed to open metadata file for writing: " << filename << "\n";
    return;
  }

  json << "{\n";
  json << "  \"driver\": \"odbc\",\n";
  json << "  \"driver_type\": \"" << driver_type << "\",\n";
  json << "  \"driver_version\": \"" << driver_version << "\",\n";
  json << "  \"build_rust_version\": \"" << build_rust_version << "\",\n";
  json << "  \"runtime_language_version\": \"NA\",\n";
  json << "  \"server_version\": \"" << server_version << "\",\n";
  json << "  \"architecture\": \"" << architecture << "\",\n";
  json << "  \"os\": \"" << os << "\",\n";
  json << "  \"run_timestamp\": " << timestamp << "\n";
  json << "}\n";

  json.close();
  std::cout << "✓ Run metadata saved to: " << filename << "\n";
}
