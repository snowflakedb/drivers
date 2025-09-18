#ifndef PUT_GET_UTILS_HPP
#define PUT_GET_UTILS_HPP

#include <sql.h>
#include <sqlext.h>
#include <stdio.h>
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
#include <stdexcept>
#include <string>
#include <vector>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "macros.hpp"

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
static constexpr int PUT_ROW_MESSAGE_IDX = 8;

// Indices for GET output rowset
static constexpr int GET_ROW_FILE_IDX = 1;
static constexpr int GET_ROW_SIZE_IDX = 2;
static constexpr int GET_ROW_STATUS_IDX = 3;
static constexpr int GET_ROW_MESSAGE_IDX = 4;

// Create a temporary stage for a test and return its name (without leading '@')
inline std::string create_stage(Connection& conn, const std::string& stage_name) {
  std::string sql = "CREATE OR REPLACE TEMPORARY STAGE " + stage_name;
  auto stmt = conn.execute(sql);
  return stage_name;
}

// Generate a random hex string for temporary directory names
inline std::string random_hex(size_t num_bytes = 8) {
  std::random_device rd;
  std::mt19937_64 gen(rd());
  std::uniform_int_distribution<uint64_t> dist(0, UINT64_MAX);

  std::stringstream ss;
  const char* hex = "0123456789abcdef";
  for (size_t i = 0; i < num_bytes; ++i) {
    uint8_t v = static_cast<uint8_t>(dist(gen) & 0xFF);
    ss << hex[(v >> 4) & 0x0F] << hex[v & 0x0F];
  }
  return ss.str();
}

// Write a text file with given content and return the path
inline std::filesystem::path write_text_file(const std::filesystem::path& dir,
                                             const std::string& filename,
                                             const std::string& content) {
  std::filesystem::create_directories(dir);
  std::filesystem::path p = dir / filename;
  std::ofstream ofs(p, std::ios::binary);
  ofs << content;
  ofs.close();
  return p;
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
  std::vector<unsigned char> compressed((std::istreambuf_iterator<char>(ifs)),
                                        std::istreambuf_iterator<char>());

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
  NEW_DRIVER_ONLY("BC#2: Compression type is now returned in uppercase") {
    CHECK(compression_type == expected_compression_type);
  }
  OLD_DRIVER_ONLY("BC#2: Compression type is now returned in uppercase") {
    std::string exp_comp_type_lower = expected_compression_type;
    std::transform(exp_comp_type_lower.begin(), exp_comp_type_lower.end(),
                   exp_comp_type_lower.begin(), [](unsigned char c) { return std::tolower(c); });
    CHECK(compression_type == exp_comp_type_lower);
  }
}

}  // namespace pg_utils

#endif  // PUT_GET_UTILS_HPP
