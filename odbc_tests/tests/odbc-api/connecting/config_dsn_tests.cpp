#ifdef _WIN32

#include <Windows.h>

#include <cstdlib>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#define ODBC_ADD_DSN 1
#define ODBC_CONFIG_DSN 2
#define ODBC_REMOVE_DSN 3

using ConfigDriverFn = int(__stdcall*)(HWND, WORD, LPCSTR, LPCSTR, LPSTR, WORD, WORD*);
using ConfigDSNWFn = int(__stdcall*)(HWND, WORD, LPCWSTR, LPCWSTR);
using ConfigDSNFn = int(__stdcall*)(HWND, WORD, LPCSTR, LPCSTR);

static std::string get_driver_path() {
  const char* path = std::getenv("DRIVER_PATH");
  if (path == nullptr || path[0] == '\0') {
    FAIL("DRIVER_PATH environment variable not set");
  }
  return path;
}

static std::vector<wchar_t> build_attrs_w(const std::vector<std::pair<std::string, std::string>>& pairs) {
  std::vector<wchar_t> buf;
  for (const auto& [k, v] : pairs) {
    std::wstring entry(k.begin(), k.end());
    entry += L'=';
    std::wstring val(v.begin(), v.end());
    entry += val;
    buf.insert(buf.end(), entry.begin(), entry.end());
    buf.push_back(L'\0');
  }
  buf.push_back(L'\0');
  return buf;
}

class DynLib {
  HMODULE handle_ = nullptr;

 public:
  DynLib(const char* name) {
    handle_ = LoadLibraryA(name);
    REQUIRE(handle_ != nullptr);
  }

  ~DynLib() {
    if (handle_) FreeLibrary(handle_);
  }

  DynLib(const DynLib&) = delete;
  DynLib& operator=(const DynLib&) = delete;

  template <typename T>
  T get(const char* name) {
    auto fn = reinterpret_cast<T>(GetProcAddress(handle_, name));
    REQUIRE(fn != nullptr);
    return fn;
  }
};

class DriverDll : public DynLib {
 public:
  DriverDll() : DynLib(get_driver_path().c_str()) {}
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
// ConfigDSNW error-handling tests (no registry interaction)
// ============================================================================

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
// ConfigDSN (ANSI) error-handling tests
// ============================================================================

TEST_CASE("ConfigDSN (ANSI): returns FALSE with NULL attributes", "[odbc-api][setup-dll][config-dsn]") {
  DriverDll dll;
  auto config_dsn = dll.get<ConfigDSNFn>("ConfigDSN");

  int ret = config_dsn(nullptr, ODBC_ADD_DSN, "Snowflake ODBC UD", nullptr);
  REQUIRE(ret == 0);
}

#endif  // _WIN32
