#include <catch2/catch_test_macros.hpp>

#include "../../../common/include/Connection.hpp"

#ifdef _WIN32
#include <cwchar>

// clang-format off: tlhelp32.h needs windows.h included first.
#include <windows.h>
#include <tlhelp32.h>
// clang-format on

// Counts this process's threads named "crl-*": the driver's CRL worker and
// refresher threads. Returns a negative value when the snapshot or a thread
// query fails, so a missing name is not treated as "not a CRL thread".
static int count_crl_threads() {
  int count = 0;
  HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
  if (snapshot == INVALID_HANDLE_VALUE) {
    return -1;
  }
  THREADENTRY32 entry = {};
  entry.dwSize = sizeof entry;
  BOOL ok = Thread32First(snapshot, &entry);
  if (!ok) {
    CloseHandle(snapshot);
    return -1;
  }
  for (; ok; ok = Thread32Next(snapshot, &entry)) {
    if (entry.th32OwnerProcessID != GetCurrentProcessId()) {
      continue;
    }
    HANDLE thread = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, FALSE, entry.th32ThreadID);
    if (thread == nullptr) {
      continue;
    }
    PWSTR name = nullptr;
    HRESULT hr = GetThreadDescription(thread, &name);
    if (FAILED(hr) || name == nullptr) {
      if (name != nullptr) {
        LocalFree(name);
      }
      CloseHandle(thread);
      CloseHandle(snapshot);
      return -1;
    }
    if (std::wcsncmp(name, L"crl-", 4) == 0) {
      ++count;
    }
    LocalFree(name);
    CloseHandle(thread);
  }
  CloseHandle(snapshot);
  return count;
}
#endif

// Scenario: Should connect and select with CRL enabled
TEST_CASE("Should connect and select with CRL enabled") {
  // Given Snowflake client is logged in
  auto connection_string = get_connection_string();
  // And CRL is enabled
  connection_string += "CRL_MODE=ENABLED;";

  {
    // When Query "SELECT 1" is executed
    Connection conn(connection_string);
    auto stmt = conn.execute_fetch("SELECT 1");

    // Then the request attempt should be successful
    SQLLEN value = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &value, 0, nullptr);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(value == 1);
#ifdef _WIN32
    // The 3.x driver has no threads named crl-*. This is a 4.x Windows unload check.
    if (get_driver_type() == DRIVER_TYPE::NEW) {
      int live = count_crl_threads();
      REQUIRE(live >= 1);
    }
#endif
  }

#ifdef _WIN32
  if (get_driver_type() == DRIVER_TYPE::NEW) {
    int after = count_crl_threads();
    REQUIRE(after == 0);
  }
#endif
}
