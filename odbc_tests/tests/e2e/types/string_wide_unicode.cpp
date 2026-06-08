#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"

namespace {

// Execute `sql` (UTF-8) by transcoding it to a DM-encoded wide buffer
// and calling `SQLExecDirectW`. Going through the wide entry point
// avoids iODBC's narrow→wide auto-conversion, which transcodes via
// Latin-1 and would mangle any non-ASCII bytes in `sql` before they
// reach the driver.
StatementHandleWrapper exec_wide_fetch(Connection& conn, const std::string& utf8_sql) {
  auto stmt = conn.executew(sf::wide::utf8_to_utf32(utf8_sql));
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  return stmt;
}

// One-shot SQLGetData into a generously-sized buffer, decoded to UTF-32.
std::u32string fetch_wide(StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  std::vector<SQLWCHAR> buf(8192, 0);
  SQLLEN ind = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_WCHAR, buf.data(),
                             static_cast<SQLLEN>(buf.size() * sizeof(SQLWCHAR)), &ind);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  REQUIRE(ind >= 0);
  return sf::wide::decode_wide(buf.data(), static_cast<size_t>(ind) / sf::wide::wchar_byte_size());
}

// "A\u4F60\u597D\U0001F680" = "A你好🚀". Written as raw UTF-8 bytes so
// the bytes the driver receives are identical regardless of the source
// encoding the compiler thinks the file is in.
const char kMixedScripts[] = "A\xE4\xBD\xA0\xE5\xA5\xBD\xF0\x9F\x9A\x80";

// "AB\U0001F680CD" - splits naturally on either side of the
// supplementary-plane code point. Under UTF-16 the rocket occupies two
// units (a surrogate pair); under UTF-32 it occupies one. With a
// 3-WCHAR get buffer (writable = 2) the chunk boundary lands right on
// the surrogate pair under UTF-16, which is exactly where the no-split
// guard fires.
const char kSurrogateStraddle[] =
    "AB\xF0\x9F\x9A\x80"
    "CD";

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should round-trip ASCII + BMP + supplementary plane "
                 "via SQL_C_WCHAR",
                 "[c_wchar][unicode][utf32]") {
  // Given a Unicode literal mixing ASCII, a BMP CJK code point, and a
  // non-BMP code point.
  std::string sql = "SELECT '";
  sql.append(kMixedScripts);
  sql.append("'");

  // When the value is fetched as SQL_C_WCHAR (via SQLExecDirectW so the
  // SQL itself is delivered to the driver as a Unicode string, not via
  // iODBC's Latin-1 narrow-to-wide auto-conversion).
  auto stmt = exec_wide_fetch(conn, sql);

  // Then the decoded code-point sequence matches the source byte-for-byte.
  CHECK(fetch_wide(stmt, 1) == sf::wide::utf8_to_utf32(kMixedScripts));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should not split a code point across chunked SQLGetData calls",
                 "[c_wchar][unicode][utf32][chunked]") {
  // Given a string whose middle code point straddles the chunk boundary
  // under UTF-16 (surrogate pair) and lands on it cleanly under UTF-32.
  std::string sql = "SELECT '";
  sql.append(kSurrogateStraddle);
  sql.append("'");
  auto stmt = exec_wide_fetch(conn, sql);

  // When SQLGetData is called repeatedly with a 3-WCHAR buffer. Under
  // UTF-16 the writable region is 2 units, exactly the size of the
  // surrogate pair: the no-split guard must keep it together. Under
  // UTF-32 the writable region holds two code points.
  std::u32string accumulated;
  SQLWCHAR buf[3] = {};
  SQLRETURN ret = SQL_SUCCESS;
  bool more = true;
  while (more) {
    std::memset(buf, 0, sizeof(buf));
    SQLLEN ind = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buf, sizeof(buf), &ind);
    if (ret == SQL_NO_DATA) {
      break;
    }
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    // The new driver always reports a non-negative byte count (BD#23). The
    // old driver may report SQL_NO_TOTAL on a truncated SQL_C_WCHAR string
    // chunk because its indicator behavior is platform- and target-type
    // dependent (see odbc_tests/BehaviorDifferences.yaml BD#23).
    NEW_DRIVER_ONLY("BD#23") { REQUIRE(ind >= 0); }
    OLD_DRIVER_ONLY("BD#23") { REQUIRE((ind >= 0 || ind == SQL_NO_TOTAL)); }

    // Decode the buffer up to (but not including) the trailing NUL. The
    // driver always NUL-terminates within the buffer, so finding the
    // first zero gives us the chunk length without having to reason
    // about whether `ind` reports the chunk size or the
    // still-remaining size (DMs differ).
    size_t units = 0;
    const size_t max_units = sizeof(buf) / sizeof(SQLWCHAR);
    while (units < max_units && buf[units] != 0) {
      ++units;
    }
    auto decoded = sf::wide::decode_wide(buf, units);

    // No chunk may end on an unpaired surrogate: that is the invariant
    // the no-split guard maintains.
    for (char32_t cp : decoded) {
      REQUIRE_FALSE((cp >= 0xD800 && cp <= 0xDFFF));
    }
    accumulated += decoded;
    more = (ret == SQL_SUCCESS_WITH_INFO);
  }

  // Then the concatenation of every chunk equals the source string.
  CHECK(accumulated == sf::wide::utf8_to_utf32(kSurrogateStraddle));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch empty string as SQL_C_WCHAR without error",
                 "[c_wchar][unicode][utf32][empty]") {
  // Given a server-side empty string.
  auto stmt = exec_wide_fetch(conn, "SELECT ''");

  // When the value is fetched into a wide buffer.
  SQLWCHAR buf[4] = {0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF};
  SQLLEN ind = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buf, sizeof(buf), &ind);

  // Then SQLGetData succeeds, the indicator reports zero payload bytes,
  // and the buffer is NUL-terminated at offset 0.
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  CHECK(ind == 0);
  CHECK(buf[0] == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should bind SQL_C_WCHAR with supplementary plane "
                 "and read back",
                 "[c_wchar][unicode][utf32][bind]") {
  // Given a Unicode source mixing ASCII, BMP, and supplementary plane,
  // and a TEMPORARY table to hold the round-tripped value. The SQL
  // setup uses ASCII only so it can travel through narrow `execute`
  // without iODBC's Latin-1 auto-conversion biting.
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  const std::u32string source = sf::wide::utf8_to_utf32(kMixedScripts);

  // When the param is bound as SQL_C_WCHAR with SQL_NTS and inserted.
  auto wide = sf::wide::encode_wide(source);
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);
  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 200, 0, wide.data(),
                                static_cast<SQLLEN>(wide.size() * sizeof(SQLWCHAR)), &ind),
               stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the server-side value round-trips back through SQL_C_WCHAR.
  auto fetch_stmt = exec_wide_fetch(conn, "SELECT col FROM t");
  CHECK(fetch_wide(fetch_stmt, 1) == source);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind empty SQL_C_WCHAR string with SQL_NTS",
                 "[c_wchar][unicode][utf32][bind][empty]") {
  // Given an empty wide string represented as a single NUL terminator.
  // Pre-fix, `read_wide_string_in` rejected a length of `0` even when
  // the buffer was a valid NUL-terminated empty string; this insert
  // would have failed with a wide-character validation error.
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  SQLWCHAR empty[1] = {0};

  // When the empty wide string is bound with SQL_NTS and inserted.
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS), stmt);
  SQLLEN ind = SQL_NTS;
  REQUIRE_ODBC(SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 200, 0, empty,
                                sizeof(empty), &ind),
               stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the row is stored as the empty string.
  auto fetch_stmt = exec_wide_fetch(conn, "SELECT col FROM t");
  SQLLEN read_ind = -1;
  SQLWCHAR out[4] = {0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF};
  SQLRETURN ret = SQLGetData(fetch_stmt.getHandle(), 1, SQL_C_WCHAR, out, sizeof(out), &read_ind);
  REQUIRE_THAT(OdbcResult(ret, fetch_stmt), OdbcMatchers::Succeeded());
  CHECK(read_ind == 0);
  CHECK(out[0] == 0);
}
