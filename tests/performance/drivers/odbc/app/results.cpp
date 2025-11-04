#include "results.h"

#include <sys/utsname.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>

void write_csv_results(const std::vector<TestResult>& results, const std::string& filename) {
  std::filesystem::path filepath(filename);
  if (filepath.has_parent_path()) {
    std::filesystem::create_directories(filepath.parent_path());
  }

  std::ofstream csv(filename);
  if (!csv.is_open()) {
    std::cerr << "ERROR: Failed to open file for writing: " << filename << "\n";
    return;
  }

  csv << "query_time_s,fetch_time_s\n";

  for (const auto& r : results) {
    csv << std::fixed << std::setprecision(6) << r.query_time_s << "," << r.fetch_time_s << "\n";
  }

  csv.close();
}

std::string get_architecture() {
  struct utsname sys_info;
  if (uname(&sys_info) == 0) {
    std::string machine = sys_info.machine;

    if (machine == "x86_64" || machine == "amd64") {
      return "x86_64";
    } else if (machine == "aarch64" || machine == "arm64") {
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

  std::ofstream json(filename);
  if (!json.is_open()) {
    std::cerr << "ERROR: Failed to open metadata file for writing: " << filename << "\n";
    return;
  }

  json << "{\n";
  json << "  \"driver\": \"odbc\",\n";
  json << "  \"driver_type\": \"" << driver_type << "\",\n";
  json << "  \"driver_version\": \"" << driver_version << "\",\n";
  json << "  \"server_version\": \"" << server_version << "\",\n";
  json << "  \"architecture\": \"" << architecture << "\",\n";
  json << "  \"os\": \"" << os << "\",\n";
  json << "  \"run_timestamp\": " << timestamp << "\n";
  json << "}\n";

  json.close();
  std::cout << "✓ Run metadata saved to: " << filename << "\n";
}
