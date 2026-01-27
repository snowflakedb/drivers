#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include <atomic>
#include <thread>
#include <vector>

#include "compatibility.hpp"
#include "Connection.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

// ============================================================================
// SQLConnect - Success Cases
// ============================================================================
// NOTE: SQLConnect requires a configured DSN in odbc.ini, not a raw hostname
// Most tests focus on error conditions that don't require a real DSN

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Completes without crashing with valid parameters", "[odbc-api][connect][connecting]") {
  // Use a non-existent DSN - should fail gracefully with IM002
  const std::string dsn = "NonExistentDSN";
  const std::string uid = "testuser";
  const std::string pwd = "testpwd";

  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(uid.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(pwd.c_str())), SQL_NTS);

  // Should fail with IM002 (DSN not found)
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Accepts explicit string lengths", "[odbc-api][connect][connecting]") {
  const std::string dsn = "TestDSN";
  const std::string uid = "user";
  const std::string pwd = "cred";

  // Connect with explicit lengths instead of SQL_NTS
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())),
                   static_cast<SQLSMALLINT>(dsn.length()),
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(uid.c_str())),
                   static_cast<SQLSMALLINT>(uid.length()),
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(pwd.c_str())),
                   static_cast<SQLSMALLINT>(pwd.length()));

  // Should fail with IM002 (DSN not found) - the important thing is it doesn't crash with explicit lengths
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Accepts NULL credentials", "[odbc-api][connect][connecting]") {
  const std::string dsn = "TestDSN";

  // Connect with NULL UID and PWD parameters
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);

  // Should fail (DSN doesn't exist) but accepts NULL credentials
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLConnect - Error Cases: Invalid Handle
// ============================================================================

TEST_CASE("SQLConnect: SQL_INVALID_HANDLE - NULL connection handle", "[odbc-api][connect][connecting][error]") {
  const auto params = get_test_parameters("testconnection");
  const std::string server = params.at("SNOWFLAKE_TEST_HOST").get<std::string>();

  const SQLRETURN ret =
      SQLConnect(SQL_NULL_HDBC, reinterpret_cast<SQLCHAR*>(const_cast<char*>(server.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLConnect: SQL_INVALID_HANDLE - Invalid handle type", "[odbc-api][connect][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  const auto params = get_test_parameters("testconnection");
  const std::string server = params.at("SNOWFLAKE_TEST_HOST").get<std::string>();

  // Try to use ENV handle as DBC handle
  ret = SQLConnect(env, reinterpret_cast<SQLCHAR*>(const_cast<char*>(server.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);

  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQLConnect - Error Cases: IM002 Data Source Not Found
// ============================================================================
// NOTE: Most connection errors with SQLConnect will be IM002 (DSN not found)
// unless a real DSN is configured. Testing 28000 would require a configured DSN.

// ============================================================================
// SQLConnect - Error Cases: IM002 Data Source Not Found
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: IM002 - Non-existent DSN", "[odbc-api][connect][connecting][error]") {
  // Use non-existent DSN
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("INVALID_DSN_DOES_NOT_EXIST")), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>("user")), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>("pwd")), SQL_NTS);

  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLConnect - Error Cases: HY090 Invalid String or Buffer Length
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: HY090 - Negative server name length", "[odbc-api][connect][connecting][error]") {
  const auto params = get_test_parameters("testconnection");
  const std::string server = params.at("SNOWFLAKE_TEST_HOST").get<std::string>();

  // Use negative length (invalid, should be >= 0 or SQL_NTS)
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(server.c_str())), -5, nullptr, 0, nullptr, 0);

  REQUIRE_EXPECTED_ERROR(ret, "HY090", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLConnect: HY090 - Negative username length", "[odbc-api][connect][connecting][error]") {
  const std::string dsn = "TestDSN";
  const std::string user = "testuser";

  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(user.c_str())), -3, nullptr, 0);

  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  // DM-dependent: unixODBC may validate lengths first (HY090) or check DSN first (IM002)
  REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
}

TEST_CASE_METHOD(DbcFixture, "SQLConnect: HY090 - Negative PWD length", "[odbc-api][connect][connecting][error]") {
  const std::string dsn = "TestDSN";
  const std::string uid = "testuser";
  const std::string pwd = "testpwd";

  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(uid.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(pwd.c_str())), -2);

  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  // DM-dependent: unixODBC may validate lengths first (HY090) or check DSN first (IM002)
  REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
}

// ============================================================================
// SQLConnect - Error Cases: IM010 or HY090 Data Source Name Too Long
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: IM010/HY090 - Data source name too long", "[odbc-api][connect][connecting][error]") {
  // Create a very long data source name (> SQL_MAX_DSN_LENGTH which is typically 32)
  // Per ODBC spec: IM010 = "*ServerName was longer than SQL_MAX_DSN_LENGTH characters"
  const std::string long_dsn(300, 'A');

  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(long_dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);

  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  // DM-dependent: IM010 (DSN too long per ODBC spec) or HY090 (generic invalid length)
  REQUIRE((records[0].sqlState == "IM010" || records[0].sqlState == "HY090"));
}

// ============================================================================
// SQLConnect - Error Cases: Default DSN Handling
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: NULL DSN uses default data source", "[odbc-api][connect][connecting][error]") {
  // NULL DSN triggers default data source lookup per ODBC spec
  const SQLRETURN ret = SQLConnect(dbc_handle(), nullptr, 0, nullptr, 0, nullptr, 0);

  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLConnect - API Validation (Lifecycle)
// ============================================================================
// NOTE: These tests validate API behavior without requiring configured DSNs

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Can be called multiple times on same handle after disconnect",
          "[odbc-api][connect][connecting][lifecycle]") {
  const std::string dsn = "TestDSN";

  // Multiple attempts (all will fail with IM002, but the API should handle it)
  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
    REQUIRE(ret == SQL_ERROR);  // DSN doesn't exist
  }
}

TEST_CASE_METHOD(EnvFixture, "SQLConnect: Multiple connection handles from same environment",
          "[odbc-api][connect][connecting][concurrent]") {
  constexpr int NUM_CONNECTIONS = 3;
  SQLHDBC connections[NUM_CONNECTIONS];

  const std::string dsn = "TestDSN";

  // Allocate multiple connection handles
  for (auto & connection : connections) {
    connection = SQL_NULL_HDBC;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &connection);
    REQUIRE(ret == SQL_SUCCESS);

    // Try to connect (will fail, but validates API accepts multiple handles)
    ret = SQLConnect(connection, reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0,
                     nullptr, 0);
    REQUIRE(ret == SQL_ERROR);  // DSN doesn't exist
  }

  // Clean up
  for (const auto & connection : connections) {
    SQLFreeHandle(SQL_HANDLE_DBC, connection);
  }
}

// ============================================================================
// SQLConnect - Edge Cases
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Empty string parameters", "[odbc-api][connect][connecting][edge]") {
  // Connect with empty server name
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("")), 0, nullptr, 0, nullptr, 0);

  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Zero-length strings with SQL_NTS", "[odbc-api][connect][connecting][edge]") {
  // Empty strings with SQL_NTS should be treated as empty
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("")), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>("")), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>("")), SQL_NTS);

  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLConnect - Connection Attribute Tests
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: SQL_ATTR_LOGIN_TIMEOUT can be set before connect", "[odbc-api][connect][connecting][attributes]") {
  // Set login timeout to 5 seconds per Microsoft example code
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_LOGIN_TIMEOUT, reinterpret_cast<SQLPOINTER>(5), 0);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify the attribute was set
  SQLUINTEGER timeout = 0;
  ret = SQLGetConnectAttr(dbc_handle(), SQL_ATTR_LOGIN_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  REQUIRE(timeout == 5);

  // Connect attempt (will fail with IM002, validates attribute setting doesn't break connect)
  const std::string dsn = "NonExistentDSN";
  ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLConnect - Handle Cleanup Verification
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLConnect: Verify proper cleanup after failed connection", "[odbc-api][connect][connecting][cleanup]") {
  const std::string dsn = "NonExistentDSN";

  // Attempt connection that will fail
  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_ERROR);

  // Per ODBC spec: SQLDisconnect should not be called if connection was never established
  // But SQLFreeHandle should still work (fixture will handle cleanup)
}

// ============================================================================
// SQLConnect - DSN-Based Integration Tests
// ============================================================================
// These tests require a configured DSN (set up via scripts/setup_test_dsn.sh)
// Tag with [.integration] to skip by default - run with: ctest -R integration
// Or set ODBCSYSINI/ODBCINI to enable these tests

// This test verifies DSN configuration and acts as a gate for other integration tests
TEST_CASE("SQLConnect: DSN configuration check", "[odbc-api][connect][dsn][integration][!mayfail]") {
  // This test explicitly checks if DSN is configured and reports clearly

  const auto config = DataSourceConfig::Snowflake().install();

  // Verify the DSN name has correct format (with unique suffix for parallel isolation)
  const std::string dsn = config.dsn_name();
  REQUIRE(dsn.rfind("Snowflake_", 0) == 0);  // C++17 compatible starts_with
  REQUIRE(dsn.length() > 10);  // Has suffix
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLConnect: Basic DSN connection succeeds", "[odbc-api][connect][dsn][integration]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string dsn = config.value().dsn_name();

  // Credentials are in odbc.ini, pass NULL for UID/PWD
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify connection by executing a query
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  ret = SQLFetch(stmt);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLConnect: 08002 - Connection already open", "[odbc-api][connect][dsn][integration][error]") {

  const std::string dsn = config.value().dsn_name();

  // First connection must succeed
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Try to connect again without disconnecting - should fail with 08002
  // Per ODBC spec: "The specified ConnectionHandle had already been used to establish
  // a connection with a data source, and the connection was still open"
  ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "08002", dbc_handle(), SQL_HANDLE_DBC);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLConnect: 28000 - Invalid authorization specification",
          "[odbc-api][connect][dsn][integration][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Use DSN without credentials and provide invalid ones
  // Note: Snowflake driver returns 28000 for authentication failures.
  // ODBC spec allows 28000, 08001, 08004, or HY000, but Snowflake consistently uses 28000.
  const std::string dsn = config.value().dsn_name();
  const std::string bad_uid = "invalid_user_xyz";
  const std::string bad_pwd = "invalid_cred_xyz";

  const SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(bad_uid.c_str())), SQL_NTS,
                   reinterpret_cast<SQLCHAR*>(const_cast<char*>(bad_pwd.c_str())), SQL_NTS);

  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLConnect: SQL_SUCCESS_WITH_INFO has retrievable diagnostics",
          "[odbc-api][connect][dsn][integration]") {

  const std::string dsn = config.value().dsn_name();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  if (ret == SQL_SUCCESS_WITH_INFO) {
    auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
    REQUIRE(!records.empty());
    // Warning SQLSTATEs start with "01"
    REQUIRE(records[0].sqlState.substr(0, 2) == "01");
  }

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLConnect: Disconnect and reconnect cycle", "[odbc-api][connect][dsn][integration][lifecycle]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string dsn = config.value().dsn_name();

  constexpr int CYCLES = 3;
  for (int i = 0; i < CYCLES; i++) {
    SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

    SQLHSTMT stmt = SQL_NULL_HSTMT;
    ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
    CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLDisconnect(dbc_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
}

TEST_CASE_METHOD(EnvDefaultDSNFixture, "SQLConnect: Multiple concurrent connections", "[odbc-api][connect][dsn][integration][concurrent]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  constexpr int NUM_CONNECTIONS = 3;
  SQLHDBC connections[NUM_CONNECTIONS];

  const std::string dsn = config.value().dsn_name();

  // Allocate and connect all connections
  for (auto & connection : connections) {
    connection = SQL_NULL_HDBC;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &connection);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLConnect(connection, reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0,
                     nullptr, 0);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  }

  // Verify all connections are independent by executing queries
  for (int i = 0; i < NUM_CONNECTIONS; i++) {
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, connections[i], &stmt);
    REQUIRE(ret == SQL_SUCCESS);

    std::string query = "SELECT " + std::to_string(i + 1);
    ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>(query.c_str())), SQL_NTS);
    CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    REQUIRE(ret == SQL_SUCCESS);
  }

  // Clean up
  for (auto & connection : connections) {
    SQLRETURN ret = SQLDisconnect(connection);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLFreeHandle(SQL_HANDLE_DBC, connection);
    REQUIRE(ret == SQL_SUCCESS);
  }
}

// Concurrent connection test with threads
static void dsn_connection_thread(const std::string& dsn, std::atomic<SQLRETURN>* result, const int iterations) {
  for (int i = 0; i < iterations; i++) {
    constexpr int MAX_RETRIES = 3;
    SQLHENV env = SQL_NULL_HENV;
    SQLHDBC dbc = SQL_NULL_HDBC;

    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
    if (ret != SQL_SUCCESS) {
      result->store(ret, std::memory_order_relaxed);
      return;
    }

    ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    if (ret != SQL_SUCCESS) {
      result->store(ret, std::memory_order_relaxed);
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
    if (ret != SQL_SUCCESS) {
      result->store(ret, std::memory_order_relaxed);
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    // Retry logic for driver loading issues
    int retry = 0;
    do {
      ret = SQLConnect(dbc, reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0,
                       nullptr, 0);
      if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
        break;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    } while (++retry < MAX_RETRIES);

    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      // Verify connection works with a query
      SQLHSTMT stmt = SQL_NULL_HSTMT;
      if (SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt) == SQL_SUCCESS) {
        SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
      }
      SQLDisconnect(dbc);
      // Don't overwrite previous success - only store on last iteration or failure
      if (i == iterations - 1) {
        result->store(SQL_SUCCESS, std::memory_order_relaxed);
      }
    } else {
      // Connection failed, store the error and stop
      result->store(ret, std::memory_order_relaxed);
      SQLFreeHandle(SQL_HANDLE_DBC, dbc);
      SQLFreeHandle(SQL_HANDLE_ENV, env);
      return;
    }

    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
  }
}

TEST_CASE("SQLConnect: Threaded concurrent connections", "[odbc-api][connect][dsn][integration][concurrent][threads]") {
  const auto config = DataSourceConfig::Snowflake().install();

  const std::string dsn = config.dsn_name();
  constexpr int NUM_THREADS = 3;

  // First, make a single connection to ensure driver is loaded
  {
    SQLHENV env = SQL_NULL_HENV;
    SQLHDBC dbc = SQL_NULL_HDBC;

    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLConnect(dbc, reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn.c_str())), SQL_NTS, nullptr, 0, nullptr, 0);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
  }

  // Now run threaded connections
  std::atomic<SQLRETURN> results[NUM_THREADS];
  std::vector<std::thread> threads;

  for (std::atomic<SQLRETURN>& result : results) {
    constexpr int ITERATIONS_PER_THREAD = 2;
    result.store(SQL_SUCCESS, std::memory_order_relaxed);
    threads.emplace_back(dsn_connection_thread, dsn, &result, ITERATIONS_PER_THREAD);
  }

  for (auto& t : threads) {
    t.join();
  }

  // Verify all threads completed successfully
  for (const std::atomic<SQLRETURN>& result : results) {
    const SQLRETURN value = result.load(std::memory_order_acquire);
    REQUIRE((value == SQL_SUCCESS || value == SQL_SUCCESS_WITH_INFO));
  }
}
