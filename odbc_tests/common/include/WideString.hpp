#ifndef WIDE_STRING_HPP
#define WIDE_STRING_HPP

#include <sql.h>
#include <sqltypes.h>

#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace sf::wide {

// Width of one DM-side `SQLWCHAR` in bytes. Always 2 (UTF-16: unixODBC,
// Windows) or 4 (UTF-32: iODBC on UNIX).
constexpr std::size_t wchar_byte_size() { return sizeof(SQLWCHAR); }

// True when the DM uses 4-byte SQLWCHAR (i.e. iODBC). Compile-time
// constant; useful for `if constexpr` branches in test code.
constexpr bool is_wide_utf32() { return sizeof(SQLWCHAR) == 4; }

// Number of DM-side code units before the first NUL in a wide
// C-style string. Mirrors `wcslen` but on `SQLWCHAR`, which is not
// always `wchar_t`.
inline std::size_t wide_strlen(const SQLWCHAR* buf) {
  std::size_t n = 0;
  while (buf[n] != 0)
    ++n;
  return n;
}

// Decode `units` DM-side code units to a sequence of Unicode code points.
// Under UTF-32 every unit is already a full code point. Under UTF-16
// well-formed surrogate pairs are recombined; unpaired surrogates are
// preserved as-is so callers can detect them.
inline std::u32string decode_wide(const SQLWCHAR* buf, std::size_t units) {
  std::u32string out;
  out.reserve(units);
  if constexpr (is_wide_utf32()) {
    for (std::size_t i = 0; i < units; ++i) {
      out.push_back(static_cast<char32_t>(buf[i]));
    }
    return out;
  } else {
    for (std::size_t i = 0; i < units; ++i) {
      auto hi = static_cast<char32_t>(static_cast<std::uint32_t>(buf[i]) & 0xFFFFu);
      if (hi >= 0xD800 && hi <= 0xDBFF && i + 1 < units) {
        auto lo = static_cast<char32_t>(static_cast<std::uint32_t>(buf[i + 1]) & 0xFFFFu);
        if (lo >= 0xDC00 && lo <= 0xDFFF) {
          out.push_back(0x10000u + ((hi - 0xD800u) << 10) + (lo - 0xDC00u));
          ++i;
          continue;
        }
      }
      out.push_back(hi);
    }
    return out;
  }
}

// Convenience overload that decodes a NUL-terminated wide C-style
// string. Equivalent to `decode_wide(buf, wide_strlen(buf))`.
inline std::u32string decode_wide_cstr(const SQLWCHAR* buf) { return decode_wide(buf, wide_strlen(buf)); }

// Encode one code point into `out`, returning the number of SQLWCHAR
// units written (1 under UTF-32; 1 for BMP / 2 for supplementary planes
// under UTF-16). Caller must guarantee `out` has at least 2 units of
// space.
inline std::size_t encode_one(char32_t cp, SQLWCHAR* out) {
  if constexpr (is_wide_utf32()) {
    out[0] = static_cast<SQLWCHAR>(cp);
    return 1;
  } else {
    if (cp < 0x10000) {
      out[0] = static_cast<SQLWCHAR>(cp);
      return 1;
    }
    auto v = static_cast<std::uint32_t>(cp) - 0x10000u;
    out[0] = static_cast<SQLWCHAR>(0xD800u | (v >> 10));
    out[1] = static_cast<SQLWCHAR>(0xDC00u | (v & 0x3FFu));
    return 2;
  }
}

// Encode a code-point sequence into a DM-sized SQLWCHAR buffer, NUL
// terminator included. The returned vector's size includes the trailing
// NUL; pass `buf.size() - 1` as the unit count to `SQLExecDirectW` /
// `SQLBindParameter` when you want to advertise the NUL-free length.
inline std::vector<SQLWCHAR> encode_wide(std::u32string_view src) {
  std::vector<SQLWCHAR> out(src.size() * 2 + 1, 0);
  std::size_t pos = 0;
  for (char32_t cp : src) {
    pos += encode_one(cp, out.data() + pos);
  }
  out[pos] = 0;
  out.resize(pos + 1);
  return out;
}

// Encode a Unicode code-point sequence to UTF-8. Useful for handing a
// wide-string result off to byte-oriented APIs (e.g. picojson) without
// having to thread `SQLWCHAR`-width awareness through the caller.
inline std::string utf32_to_utf8(std::u32string_view src) {
  std::string out;
  out.reserve(src.size() * 4);
  for (char32_t cp : src) {
    if (cp < 0x80) {
      out.push_back(static_cast<char>(cp));
    } else if (cp < 0x800) {
      out.push_back(static_cast<char>(0xC0 | (cp >> 6)));
      out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else if (cp < 0x10000) {
      out.push_back(static_cast<char>(0xE0 | (cp >> 12)));
      out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
      out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else {
      out.push_back(static_cast<char>(0xF0 | (cp >> 18)));
      out.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3F)));
      out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
      out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    }
  }
  return out;
}

// Decode a UTF-8 byte string to a Unicode code-point sequence. Source
// literals in tests are typically written as raw byte escapes (e.g.
// `"\xE4\xBD\xA0"` for U+4F60) so the expected `std::u32string` is
// independent of the file's source encoding. Invalid UTF-8 is silently
// skipped; tests should call `FAIL(...)` on the caller side if they
// need strict validation.
inline std::u32string utf8_to_utf32(std::string_view bytes) {
  std::u32string out;
  std::size_t i = 0;
  while (i < bytes.size()) {
    auto b = static_cast<unsigned char>(bytes[i]);
    char32_t cp = 0;
    std::size_t n = 0;
    if (b < 0x80) {
      cp = b;
      n = 1;
    } else if ((b >> 5) == 0x6) {
      cp = b & 0x1F;
      n = 2;
    } else if ((b >> 4) == 0xE) {
      cp = b & 0x0F;
      n = 3;
    } else if ((b >> 3) == 0x1E) {
      cp = b & 0x07;
      n = 4;
    } else {
      ++i;
      continue;
    }
    if (i + n > bytes.size()) break;
    for (std::size_t k = 1; k < n; ++k) {
      cp = (cp << 6) | (static_cast<unsigned char>(bytes[i + k]) & 0x3F);
    }
    out.push_back(cp);
    i += n;
  }
  return out;
}

}  // namespace sf::wide

#endif  // WIDE_STRING_HPP
