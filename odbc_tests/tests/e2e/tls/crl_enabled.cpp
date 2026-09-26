#include <catch2/catch_test_macros.hpp>

#include "../../../common/include/Connection.hpp"

#ifdef _WIN32
#include <cwchar>

// clang-format off: tlhelp32.h needs windows.h included first.
#include <windows.h>
#include <tlhelp32.h>
// clang-format on

// Counts this process's threads named "crl-*": the driver's CRL worker and
// refresher threads.
static int count_crl_threads() {
  int count = 0;
  HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
  if (snapshot == INVALID_HANDLE_VALUE) {
    return -1;
  }
  THREADENTRY32 entry = {};
  entry.dwSize = sizeof entry;
  for (BOOL ok = Thread32First(snapshot, &entry); ok; ok = Thread32Next(snapshot, &entry)) {
    if (entry.th32OwnerProcessID != GetCurrentProcessId()) {
      continue;
    }
    HANDLE thread = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, FALSE, entry.th32ThreadID);
    if (thread == nullptr) {
      continue;
    }
    PWSTR name = nullptr;
    if (SUCCEEDED(GetThreadDescription(thread, &name)) && name != nullptr) {
      if (std::wcsncmp(name, L"crl-", 4) == 0) {
        ++count;
      }
      LocalFree(name);
    }
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
    SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &value, 0, nullptr);
    REQUIRE(value == 1);
  }

#ifdef _WIN32
  // Windows-only regression check: the Driver Manager unloads the driver DLL
  // right after the last environment is freed, so any CRL thread still
  // running at that point faults as soon as it next runs.
  REQUIRE(count_crl_threads() == 0);
#endif
}
