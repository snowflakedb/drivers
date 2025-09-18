#ifndef UTILS_HPP
#define UTILS_HPP

#include <array>
#include <cctype>
#include <filesystem>
#include <stdexcept>
#include <string>

#ifdef _WIN32
#include <io.h>
#define popen _popen
#define pclose _pclose
#else
#include <cstdio>
#endif

namespace test_utils {

inline std::filesystem::path repo_root() {
  const char* cmd = "git rev-parse --show-toplevel";
#ifdef _WIN32
  FILE* pipe = _popen(cmd, "r");
#else
  FILE* pipe = popen(cmd, "r");
#endif
  if (!pipe) {
    throw std::runtime_error("Failed to determine repository root: unable to start git command");
  }

  std::array<char, 256> buffer{};
  std::string output;
  while (fgets(buffer.data(), static_cast<int>(buffer.size()), pipe) != nullptr) {
    output.append(buffer.data());
  }

#ifdef _WIN32
  int rc = _pclose(pipe);
#else
  int rc = pclose(pipe);
#endif

  while (!output.empty() && std::isspace(static_cast<unsigned char>(output.back()))) {
    output.pop_back();
  }

  if (rc == 0 && !output.empty()) {
    return std::filesystem::path(output);
  }

  throw std::runtime_error("Failed to determine repository root");
}

inline std::filesystem::path shared_test_data_dir() {
  return repo_root() / "tests" / "test_data" / "generated_test_data";
}

// Helper function to get test data file path
inline std::filesystem::path test_data_file_path(const std::string& relative_path) {
  return repo_root() / "tests" / "test_data" / relative_path;
}

}  // namespace test_utils

#endif  // UTILS_HPP
