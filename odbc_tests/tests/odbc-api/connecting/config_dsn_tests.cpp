#ifdef _WIN32

#include <Windows.h>
#include <odbcinst.h>

#include <cstdlib>
#include <random>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

using ConfigDriverFn = int(__stdcall*)(HWND, WORD, LPCSTR, LPCSTR, LPSTR, WORD, WORD*);
using ConfigDSNWFn = int(__stdcall*)(HWND, WORD, LPCWSTR, LPCWSTR);
using ConfigDSNFn = int(__stdcall*)(HWND, WORD, LPCSTR, LPCSTR);

constexpr WORD ODBC_ADD_DSN = 1;
constexpr WORD ODBC_CONFIG_DSN = 2;
constexpr WORD ODBC_REMOVE_DSN = 3;

static std::string get_driver_path() {
  const char* path = std::getenv("DRIVER_PATH");
  if (path == nullptr || path[0] == '\0') {
    FAIL("DRIVER_PATH environment variable not set");
  }
  return path;
}

static std::string random_dsn_name() {
  static constexpr char chars[] = "abcdefghijklmnopqrstuvwxyz0123456789";
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<size_t> dist(0, sizeof(chars) - 2);
  std::string suffix(8, '\0');
  for (auto& c : suffix)
    c = chars[dist(gen)];
  return "ConfigDSNTest_" + suffix;
}

static std::wstring to_wide(const std::string& s) { return std::wstring(s.begin(), s.end()); }

/// Build a double-null-terminated wide attribute string from pairs.
static std::vector<wchar_t> build_attrs_w(const std::vector<std::pair<std::string, std::string>>& pairs) {
  std::vector<wchar_t> buf;
  for (const auto& [k, v] : pairs) {
    std::wstring entry = to_wide(k + "=" + v);
    buf.insert(buf.end(), entry.begin(), entry.end());
    buf.push_back(L'\0');
  }
  buf.push_back(L'\0');
  return buf;
}

/// Build a double-null-terminated ANSI attribute string from pairs.
static std::vector<char> build_attrs_a(const std::vector<std::pair<std::string, std::string>>& pairs) {
  std::vector<char> buf;
  for (const auto& [k, v] : pairs) {
    std::string entry = k + "=" + v;
    buf.insert(buf.end(), entry.begin(), entry.end());
    buf.push_back('\0');
  }
  buf.push_back('\0');
  return buf;
}

/// Read a DSN entry value from ODBC.INI via the installer API.
static std::string read_dsn_value(const std::string& dsn, const std::string& key) {
  char buf[512] = {};
  SQLGetPrivateProfileStringA(dsn.c_str(), key.c_str(), "", buf, sizeof(buf), "odbc.ini");
  return buf;
}

/// Check if a DSN exists in ODBC.INI.
static bool dsn_exists(const std::string& dsn) {
  char buf[4096] = {};
  int len = SQLGetPrivateProfileStringA(nullptr, nullptr, "", buf, sizeof(buf), "odbc.ini");
  const char* p = buf;
  while (p < buf + len && *p != '\0') {
    if (dsn == p) return true;
    p += strlen(p) + 1;
  }
  return false;
}

class DriverDll {
  HMODULE handle_ = nullptr;

 public:
  DriverDll() {
    std::string path = get_driver_path();
    handle_ = LoadLibraryA(path.c_str());
    REQUIRE(handle_ != nullptr);
  }

  ~DriverDll() {
    if (handle_) FreeLibrary(handle_);
  }

  DriverDll(const DriverDll&) = delete;
  DriverDll& operator=(const DriverDll&) = delete;

  template <typename T>
  T get(const char* name) {
    auto fn = reinterpret_cast<T>(GetProcAddress(handle_, name));
    REQUIRE(fn != nullptr);
    return fn;
  }
};

/// RAII guard that removes a DSN from the registry on destruction,
/// ensuring cleanup even when a test assertion fails midway.
class DsnGuard {
  std::string dsn_;

 public:
  explicit DsnGuard(const std::string& dsn) : dsn_(dsn) {}
  ~DsnGuard() { SQLRemoveDSNFromIniA(dsn_.c_str()); }

  DsnGuard(const DsnGuard&) = delete;
  DsnGuard& operator=(const DsnGuard&) = delete;
};

// ============================================================================
// ConfigDriver tests
// ============================================================================

TEST_CASE("ConfigDriver returns TRUE", "[odbc-api][setup-dll][config-driver]") {
  DriverDll dll;
  auto fn = dll.get<ConfigDriverFn>("ConfigDriver");
  WORD cbOut = 0xFFFF;
  int ret = fn(nullptr, ODBC_ADD_DSN, "TestDriver", nullptr, nullptr, 0, &cbOut);
  REQUIRE(ret == 1);
  REQUIRE(cbOut == 0);
}

TEST_CASE("ConfigDriver with NULL pcbMsgOut", "[odbc-api][setup-dll][config-driver]") {
  DriverDll dll;
  auto fn = dll.get<ConfigDriverFn>("ConfigDriver");
  int ret = fn(nullptr, ODBC_REMOVE_DSN, "TestDriver", nullptr, nullptr, 0, nullptr);
  REQUIRE(ret == 1);
}

// ============================================================================
// ConfigDSNW tests (Unicode)
// ============================================================================

TEST_CASE("ConfigDSNW: add and remove a DSN", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn_w = dll.get<ConfigDSNWFn>("ConfigDSNW");
  std::string dsn = random_dsn_name();
  DsnGuard guard(dsn);
  std::wstring driver = L"Snowflake ODBC UD";

  auto attrs = build_attrs_w({{"DSN", dsn}, {"SERVER", "test.snowflake.com"}});
  int ret = config_dsn_w(nullptr, ODBC_ADD_DSN, driver.c_str(), attrs.data());
  REQUIRE(ret == 1);
  REQUIRE(dsn_exists(dsn));
  REQUIRE(read_dsn_value(dsn, "SERVER") == "test.snowflake.com");

  auto rm_attrs = build_attrs_w({{"DSN", dsn}});
  ret = config_dsn_w(nullptr, ODBC_REMOVE_DSN, driver.c_str(), rm_attrs.data());
  REQUIRE(ret == 1);
  REQUIRE_FALSE(dsn_exists(dsn));
}

TEST_CASE("ConfigDSNW: modify an existing DSN", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn_w = dll.get<ConfigDSNWFn>("ConfigDSNW");
  std::string dsn = random_dsn_name();
  DsnGuard guard(dsn);
  std::wstring driver = L"Snowflake ODBC UD";

  auto add_attrs = build_attrs_w({{"DSN", dsn}, {"SERVER", "old.snowflake.com"}, {"UID", "user1"}});
  int ret = config_dsn_w(nullptr, ODBC_ADD_DSN, driver.c_str(), add_attrs.data());
  REQUIRE(ret == 1);
  REQUIRE(read_dsn_value(dsn, "SERVER") == "old.snowflake.com");

  auto mod_attrs = build_attrs_w({{"DSN", dsn}, {"SERVER", "new.snowflake.com"}, {"UID", "user2"}});
  ret = config_dsn_w(nullptr, ODBC_CONFIG_DSN, driver.c_str(), mod_attrs.data());
  REQUIRE(ret == 1);
  REQUIRE(read_dsn_value(dsn, "SERVER") == "new.snowflake.com");
  REQUIRE(read_dsn_value(dsn, "UID") == "user2");
}

TEST_CASE("ConfigDSNW: returns FALSE with missing DSN attribute", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn_w = dll.get<ConfigDSNWFn>("ConfigDSNW");
  std::wstring driver = L"Snowflake ODBC UD";

  auto attrs = build_attrs_w({{"SERVER", "test.snowflake.com"}});
  int ret = config_dsn_w(nullptr, ODBC_ADD_DSN, driver.c_str(), attrs.data());
  REQUIRE(ret == 0);
}

TEST_CASE("ConfigDSNW: returns FALSE with NULL attributes", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn_w = dll.get<ConfigDSNWFn>("ConfigDSNW");
  std::wstring driver = L"Snowflake ODBC UD";

  int ret = config_dsn_w(nullptr, ODBC_ADD_DSN, driver.c_str(), nullptr);
  REQUIRE(ret == 0);
}

TEST_CASE("ConfigDSNW: returns FALSE for invalid request code", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn_w = dll.get<ConfigDSNWFn>("ConfigDSNW");
  std::wstring driver = L"Snowflake ODBC UD";

  auto attrs = build_attrs_w({{"DSN", "SomeDSN"}});
  int ret = config_dsn_w(nullptr, 99, driver.c_str(), attrs.data());
  REQUIRE(ret == 0);
}

// ============================================================================
// ConfigDSN tests (ANSI)
// ============================================================================

TEST_CASE("ConfigDSN (ANSI): add and remove a DSN", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn = dll.get<ConfigDSNFn>("ConfigDSN");
  std::string dsn = random_dsn_name();
  DsnGuard guard(dsn);

  auto attrs = build_attrs_a({{"DSN", dsn}, {"SERVER", "ansi.snowflake.com"}});
  int ret = config_dsn(nullptr, ODBC_ADD_DSN, "Snowflake ODBC UD", attrs.data());
  REQUIRE(ret == 1);
  REQUIRE(dsn_exists(dsn));
  REQUIRE(read_dsn_value(dsn, "SERVER") == "ansi.snowflake.com");

  auto rm_attrs = build_attrs_a({{"DSN", dsn}});
  ret = config_dsn(nullptr, ODBC_REMOVE_DSN, "Snowflake ODBC UD", rm_attrs.data());
  REQUIRE(ret == 1);
  REQUIRE_FALSE(dsn_exists(dsn));
}

TEST_CASE("ConfigDSN (ANSI): returns FALSE with NULL attributes", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn = dll.get<ConfigDSNFn>("ConfigDSN");

  int ret = config_dsn(nullptr, ODBC_ADD_DSN, "Snowflake ODBC UD", nullptr);
  REQUIRE(ret == 0);
}

#endif  // _WIN32
