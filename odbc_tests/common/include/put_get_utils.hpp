#ifndef PUT_GET_UTILS_HPP
#define PUT_GET_UTILS_HPP

#include <zlib.h>

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <random>
#include <sstream>
#include <string>
#include <vector>

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

// Read a file's full contents as raw bytes. Used to feed the gzip header
// byte-readers below.
inline std::vector<unsigned char> read_file_bytes(const std::filesystem::path& path) {
  std::ifstream ifs(path, std::ios::binary);
  REQUIRE(ifs.good());
  return std::vector<unsigned char>((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
}

// Direct accessors for the bytes inside a gzip stream's fixed 10-byte
// header preamble (RFC 1952 §2.3): ID1, ID2, CM, FLG, MTIME[4], XFL, OS.
// Every helper REQUIREs the magic / size invariants, so callers can assert
// against the returned value directly.
inline std::uint8_t gzip_flg(const std::vector<unsigned char>& bytes) {
  REQUIRE(bytes.size() >= 10);
  REQUIRE(bytes[0] == 0x1f);
  REQUIRE(bytes[1] == 0x8b);
  return bytes[3];
}

inline std::uint32_t gzip_mtime(const std::vector<unsigned char>& bytes) {
  REQUIRE(bytes.size() >= 10);
  // Little-endian per RFC 1952 §2.3.1.4.
  return static_cast<std::uint32_t>(bytes[4]) | static_cast<std::uint32_t>(bytes[5]) << 8 |
         static_cast<std::uint32_t>(bytes[6]) << 16 | static_cast<std::uint32_t>(bytes[7]) << 24;
}

inline std::uint8_t gzip_xfl(const std::vector<unsigned char>& bytes) {
  REQUIRE(bytes.size() >= 10);
  return bytes[8];
}

inline std::uint8_t gzip_os(const std::vector<unsigned char>& bytes) {
  REQUIRE(bytes.size() >= 10);
  return bytes[9];
}

// Mirrors libsnowflakeclient/deps/zlib-1.3.1/zutil.h::OS_CODE so the
// test side and the Rust `ZLIB_OS_CODE` constant in
// `sf_core::compression` resolve to the same value on every supported
// build target. When asserting against UD-ODBC and legacy ODBC gzip
// output, the OS byte must equal this value (the legacy driver's libz
// picks up the same macro at compile time).
inline std::uint8_t expected_zlib_os_code() {
#if defined(__APPLE__)
  return 19;
#elif defined(_WIN32) && !defined(__CYGWIN__)
  return 10;
#else
  return 3;  // Unix-like default
#endif
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
