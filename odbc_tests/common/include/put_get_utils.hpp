#ifndef PUT_GET_UTILS_HPP
#define PUT_GET_UTILS_HPP

#include <zlib.h>

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <random>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"

namespace pg_utils {

// Indices for LS output rowset
static constexpr int LS_ROW_NAME_IDX = 1;

// Indices for PUT output rowset
static constexpr int PUT_ROW_SOURCE_IDX = 1;
static constexpr int PUT_ROW_TARGET_IDX = 2;
static constexpr int PUT_ROW_SOURCE_SIZE_IDX = 3;
static constexpr int PUT_ROW_TARGET_SIZE_IDX = 4;
static constexpr int PUT_ROW_SOURCE_COMPRESSION_IDX = 5;
static constexpr int PUT_ROW_TARGET_COMPRESSION_IDX = 6;
static constexpr int PUT_ROW_STATUS_IDX = 7;
static constexpr int PUT_ROW_ENCRYPTION_IDX = 8;
static constexpr int PUT_ROW_MESSAGE_IDX = 9;
static constexpr int PUT_ROW_NUM_COLS = 9;

// Literal emitted in the PUT result's `message` column when the upload
// outcome is SKIPPED (overwrite=false + file exists). Mirrors
// `ODBC_PUT_MESSAGE_SKIPPED` in `sf_core::apis::database_driver_v1` and the
// legacy libsnowflakeclient `MESSAGE_SKIPPED` macro.
inline constexpr auto PUT_ROW_MESSAGE_SKIPPED = "File with same name already exists. SKIPPED";

// Indices for GET output rowset
static constexpr int GET_ROW_FILE_IDX = 1;
static constexpr int GET_ROW_SIZE_IDX = 2;
static constexpr int GET_ROW_STATUS_IDX = 3;
static constexpr int GET_ROW_ENCRYPTION_IDX = 4;
static constexpr int GET_ROW_MESSAGE_IDX = 5;
static constexpr int GET_ROW_NUM_COLS = 5;

// Generate a random hex string for temporary directory names
inline std::string random_hex(size_t num_bytes = 8) {
  std::random_device rd;
  std::mt19937_64 gen(rd());
  std::uniform_int_distribution<uint64_t> dist(0, UINT64_MAX);

  std::stringstream ss;
  for (size_t i = 0; i < num_bytes; ++i) {
    const auto hex = "0123456789abcdef";
    const auto v = static_cast<uint8_t>(dist(gen) & 0xFF);
    ss << hex[(v >> 4) & 0x0F] << hex[v & 0x0F];
  }
  return ss.str();
}

// Generate a unique stage name with random suffix for parallel test safety
inline std::string unique_stage_name(const std::string& prefix) { return prefix + "_" + random_hex(4); }

// Create a temporary stage for a test and return its name (without leading '@')
inline std::string create_stage(Connection& conn, const std::string& stage_name) {
  std::string sql = "CREATE OR REPLACE TEMPORARY STAGE " + stage_name;
  auto stmt = conn.execute(sql);
  return stage_name;
}

// RAII wrapper for temporary test directories - automatically cleans up on destruction
class TempTestDir {
 public:
  explicit TempTestDir(const std::string& prefix = "odbc_test_")
      : path_(std::filesystem::temp_directory_path() / (prefix + random_hex_internal())) {
    std::filesystem::create_directories(path_);
  }

  ~TempTestDir() {
    if (std::filesystem::exists(path_)) {
      std::error_code ec;
      std::filesystem::remove_all(path_, ec);
      // Ignore errors during cleanup - test environment may have already cleaned up
    }
  }

  // Non-copyable, movable
  TempTestDir(const TempTestDir&) = delete;
  TempTestDir& operator=(const TempTestDir&) = delete;
  TempTestDir(TempTestDir&& other) noexcept : path_(std::move(other.path_)) { other.path_.clear(); }
  TempTestDir& operator=(TempTestDir&& other) noexcept {
    if (this != &other) {
      path_ = std::move(other.path_);
      other.path_.clear();
    }
    return *this;
  }

  [[nodiscard]] const std::filesystem::path& path() const { return path_; }
  [[nodiscard]] operator const std::filesystem::path&() const { return path_; }

 private:
  std::filesystem::path path_;

  static std::string random_hex_internal(size_t num_bytes = 8) {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    std::uniform_int_distribution<uint64_t> dist(0, UINT64_MAX);

    std::stringstream ss;
    for (size_t i = 0; i < num_bytes; ++i) {
      const auto hex = "0123456789abcdef";
      const auto v = static_cast<uint8_t>(dist(gen) & 0xFF);
      ss << hex[(v >> 4) & 0x0F] << hex[v & 0x0F];
    }
    return ss.str();
  }
};

// Resolve the user's home directory: $HOME on Unix, %USERPROFILE% on Windows.
// Returns an empty path when the variable is unset so the caller can SKIP.
inline std::filesystem::path user_home_dir() {
#ifdef _WIN32
  const char* home = std::getenv("USERPROFILE");
#else
  const char* home = std::getenv("HOME");
#endif
  return (home != nullptr) ? std::filesystem::path(home) : std::filesystem::path();
}

// RAII wrapper for a uniquely-named subdirectory created directly under the
// user's home directory. Tilde-expansion tests require source files to live
// under home; this guard keeps tests hermetic. Mirrors TempTestDir, which
// always roots under the system temp directory.
class HomeTempDir {
 public:
  HomeTempDir(const std::filesystem::path& home, const std::string& prefix)
      : name_(prefix + random_hex(4)), path_(home / name_) {
    std::filesystem::create_directories(path_);
  }

  ~HomeTempDir() {
    // A destructor must not throw, so use the non-throwing overload and check
    // the result. Surface a cleanup failure via Catch2's WARN rather than bare
    // std::cerr (test-diagnostic-output), without failing the test.
    std::error_code ec;
    std::filesystem::remove_all(path_, ec);
    if (ec) {
      WARN("HomeTempDir cleanup failed for " << path_.string() << ": " << ec.message());
    }
  }

  HomeTempDir(const HomeTempDir&) = delete;
  HomeTempDir& operator=(const HomeTempDir&) = delete;

  [[nodiscard]] const std::filesystem::path& path() const { return path_; }
  [[nodiscard]] const std::string& name() const { return name_; }

 private:
  std::string name_;
  std::filesystem::path path_;
};

// Write a text file with given content and return the path
inline std::filesystem::path write_text_file(const std::filesystem::path& dir, const std::string& filename,
                                             const std::string& content) {
  std::filesystem::create_directories(dir);
  std::filesystem::path p = dir / filename;
  std::ofstream ofs(p, std::ios::binary);
  ofs << content;
  ofs.close();
  return p;
}

// On Windows, both drivers return the full absolute file path verbatim in the PUT source column.
inline std::string expected_put_source(const std::filesystem::path& file_path) {
  WINDOWS_ONLY {
    std::string s = std::filesystem::absolute(file_path).string();
    std::replace(s.begin(), s.end(), '\\', '/');
    return s;
  }
  UNIX_ONLY { return file_path.filename().string(); }
  throw std::logic_error("expected_put_source: unsupported platform");
}

// Convert a path into a URI-safe string for Snowflake file:// usage
inline std::string as_file_uri(const std::filesystem::path& p) {
  std::string s = p.string();
#ifdef _WIN32
  // Replace backslashes with forward slashes for URIs on Windows
  std::replace(s.begin(), s.end(), '\\', '/');
#endif
  return s;
}

// Simple gzip decompression utility used by tests to verify content
inline std::string decompress_gzip_file(const std::filesystem::path& gz_path) {
  std::ifstream ifs(gz_path, std::ios::binary);
  REQUIRE(ifs.good());
  std::vector<unsigned char> compressed((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());

  // Set up zlib inflate with gzip header support
  z_stream strm{};
  strm.next_in = compressed.data();
  strm.avail_in = static_cast<uInt>(compressed.size());

  int ret = inflateInit2(&strm, 16 + MAX_WBITS);
  REQUIRE(ret == Z_OK);

  std::string out;
  std::array<unsigned char, 8192> buffer{};
  do {
    strm.next_out = buffer.data();
    strm.avail_out = static_cast<uInt>(buffer.size());
    ret = inflate(&strm, Z_NO_FLUSH);
    bool inflate_ok = (ret == Z_OK) || (ret == Z_STREAM_END);
    REQUIRE(inflate_ok);
    size_t have = buffer.size() - strm.avail_out;
    out.append(reinterpret_cast<const char*>(buffer.data()), have);
  } while (ret != Z_STREAM_END);

  inflateEnd(&strm);
  return out;
}

inline void compare_compression_type(const std::string& compression_type,
                                     const std::string& expected_compression_type) {
  NEW_DRIVER_ONLY("BD#2: Compression type is now returned in uppercase") {
    CHECK(compression_type == expected_compression_type);
  }
  OLD_DRIVER_ONLY("BD#2: Compression type is now returned in uppercase") {
    std::string exp_comp_type_lower = expected_compression_type;
    std::transform(exp_comp_type_lower.begin(), exp_comp_type_lower.end(), exp_comp_type_lower.begin(),
                   [](unsigned char c) { return std::tolower(c); });
    CHECK(compression_type == exp_comp_type_lower);
  }
}

}  // namespace pg_utils

#endif  // PUT_GET_UTILS_HPP
