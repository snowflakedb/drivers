#ifndef UTILS_HPP
#define UTILS_HPP

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
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

/// Returns the QUERY_RESULT_FORMAT environment variable normalized to uppercase,
/// or an empty string if unset or empty.
inline std::string get_query_result_format() {
  const char* result_format = std::getenv("QUERY_RESULT_FORMAT");
  if (result_format == nullptr || result_format[0] == '\0') {
    return {};
  }
  std::string normalized(result_format);
  std::transform(normalized.begin(), normalized.end(), normalized.begin(),
                 [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
  return normalized;
}

inline std::string base64_encode(const std::string& input) {
  static constexpr char table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  std::string out;
  out.reserve(4 * ((input.size() + 2) / 3));
  auto* src = reinterpret_cast<const unsigned char*>(input.data());
  size_t len = input.size();
  for (size_t i = 0; i < len; i += 3) {
    uint32_t n = static_cast<uint32_t>(src[i]) << 16;
    if (i + 1 < len) n |= static_cast<uint32_t>(src[i + 1]) << 8;
    if (i + 2 < len) n |= static_cast<uint32_t>(src[i + 2]);
    out.push_back(table[(n >> 18) & 0x3F]);
    out.push_back(table[(n >> 12) & 0x3F]);
    out.push_back((i + 1 < len) ? table[(n >> 6) & 0x3F] : '=');
    out.push_back((i + 2 < len) ? table[n & 0x3F] : '=');
  }
  return out;
}

inline std::filesystem::path repo_root() {
  const char* git_root_env_value = std::getenv("GIT_ROOT");
  if (git_root_env_value != nullptr && git_root_env_value[0] != '\0') {
    return std::filesystem::path(git_root_env_value);
  }
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

/// Decrypt an encrypted PEM private key and write the unencrypted PEM to a file.
/// Shells out to the openssl CLI to avoid a compile-time dependency on libcrypto.
inline void decrypt_pem_key_to_file(const std::string& encrypted_pem, const std::string& password,
                                    const std::filesystem::path& output_path) {
  // Write encrypted PEM to a temp file so openssl can read it
  auto input_path = output_path;
  input_path += ".enc";
  {
    std::ofstream f(input_path, std::ios::binary);
    if (!f) throw std::runtime_error("Failed to write temporary encrypted key file");
    f << encrypted_pem;
  }

  std::string cmd = "openssl pkey -in " + input_path.string() + " -out " + output_path.string() +
                    " -passin pass:" + password + " 2>&1";

#ifdef _WIN32
  FILE* pipe = _popen(cmd.c_str(), "r");
#else
  FILE* pipe = popen(cmd.c_str(), "r");
#endif
  if (!pipe) {
    std::filesystem::remove(input_path);
    throw std::runtime_error("Failed to run openssl command");
  }

  std::string output;
  std::array<char, 256> buf{};
  while (fgets(buf.data(), static_cast<int>(buf.size()), pipe) != nullptr) {
    output.append(buf.data());
  }

#ifdef _WIN32
  int rc = _pclose(pipe);
#else
  int rc = pclose(pipe);
#endif
  std::filesystem::remove(input_path);

  if (rc != 0) {
    throw std::runtime_error("openssl pkey failed: " + output);
  }
}

}  // namespace test_utils

#endif  // UTILS_HPP
