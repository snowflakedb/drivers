#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <atomic>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// Helper to build connection string from DSN
std::string build_dsn_connection_string(const std::string& dsn) { return "DSN=" + dsn; }

// ============================================================================
// SQLDriverConnect - API Validation Tests (No DSN Required)
// ============================================================================
// These tests validate API behavior and error handling without needing a
// configured DSN. They test the Driver Manager's parameter validation.

TEST_CASE("SQLDriverConnect: SQL_INVALID_HANDLE - NULL connection handle",
          "[odbc-api][driverconnect][connecting][error]") {
  const SQLRETURN ret =
      SQLDriverConnect(SQL_NULL_HDBC, nullptr, sqlchar("DSN=test"), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// NOTE: Testing with fake/garbage handle pointers (e.g., 0xDEADBEEF) is unsafe
// and will cause segfaults as the driver manager dereferences the pointer.
// Such tests are intentionally omitted - ODBC does not validate pointer contents.

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: HY090 - Negative string length",
                 "[odbc-api][driverconnect][connecting][error]") {
  // Given an allocated but unconnected DBC handle
  // When SQLDriverConnect is called with a negative StringLength1 (-5, not SQL_NTS)
  const SQLRETURN ret =
      SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=test"), -5, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);

  // Then a diagnostic record is produced
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  NON_IODBC {
    // And the DM validates StringLength1 first and reports
    //   HY090 (invalid string or buffer length), or passes through to DSN lookup with IM002
    REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
  }
  IODBC_ONLY {
    // And the iODBC DM forwards to driver resolution, which fails with the
    //   ODBC 2.x driver-not-found SQLSTATE "S1090" rather than the
    //   ODBC 3.x IM003/IM002
    REQUIRE(records[0].sqlState == "S1090");
  }
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: HY090 - Negative buffer length",
                 "[odbc-api][driverconnect][connecting][error]") {
  // Given an allocated but unconnected DBC handle and an output buffer
  SQLCHAR outConnStr[256];
  SQLSMALLINT outConnStrLen = 0;

  // When SQLDriverConnect is called with a negative output BufferLength (-1)
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=NonExistentDSN"), SQL_NTS, outConnStr, -1,
                                         &outConnStrLen, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);

  // Then a diagnostic record is produced
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  NON_IODBC {
    // And the DM validates the buffer length first and reports
    //   HY090, or passes through to DSN lookup with IM002
    REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
  }
  IODBC_ONLY {
    // And the iODBC DM forwards to DSN/driver lookup, which reports the
    //   ODBC 2.x driver-not-found SQLSTATE "S1090" (not IM002)
    REQUIRE(records[0].sqlState == "S1090");
  }
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: HY110 - Invalid DriverCompletion value",
                 "[odbc-api][driverconnect][connecting][error]") {
  // Given an allocated but unconnected DBC handle
  // When SQLDriverConnect is called with an out-of-range DriverCompletion value (999)
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=test"), SQL_NTS, nullptr, 0, nullptr, 999);

  NON_IODBC {
    // And the DM validates DriverCompletion and reports
    //   HY110 (invalid driver completion)
    REQUIRE_EXPECTED_ERROR(ret, "HY110", dbc_handle(), SQL_HANDLE_DBC);
  }
  IODBC_ONLY {
    // And the iODBC DM does not validate DriverCompletion against the
    //   ODBC 3.x range; it forwards to DSN/driver lookup, which fails with
    //   the ODBC 2.x driver-not-found SQLSTATE "S1090". Only the return code
    //   is asserted because the exact failure path depends on environment.
    REQUIRE(ret == SQL_ERROR);
  }
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: IM002 - Data source not found (non-existent DSN)",
                 "[odbc-api][driverconnect][connecting][error]") {
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=NonExistentDSN12345"), SQL_NTS, nullptr, 0,
                                         nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: IM007 - No data source or driver specified with NOPROMPT",
                 "[odbc-api][driverconnect][connecting][error]") {
  // Empty connection string with SQL_DRIVER_NOPROMPT
  // Note: DM-dependent - spec says IM007, but some DMs return IM002
  const SQLRETURN ret =
      SQLDriverConnect(dbc_handle(), nullptr, sqlchar(""), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  // DM-dependent: IM007 (per spec) or IM002 (unixODBC behavior)
  REQUIRE((records[0].sqlState == "IM007" || records[0].sqlState == "IM002"));
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Empty connection string",
                 "[odbc-api][driverconnect][connecting][error]") {
  const SQLRETURN ret =
      SQLDriverConnect(dbc_handle(), nullptr, sqlchar(""), 0, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: NULL connection string",
                 "[odbc-api][driverconnect][connecting][error]") {
  // iODBC dereferences InConnectionString without a NULL check and segfaults
  // instead of returning an error; the scenario is unrunnable under iODBC.
  SKIP_IODBC(
      "iODBC DM dereferences InConnectionString without a null check and segfaults instead of returning SQL_ERROR");

  // Given an allocated but unconnected DBC handle
  // When SQLDriverConnect is called with a NULL InConnectionString
  const SQLRETURN ret =
      SQLDriverConnect(dbc_handle(), nullptr, nullptr, SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);

  // Then the DM null-checks InConnectionString and reports SQL_ERROR rather than
  //   dereferencing it (iODBC is skipped above because it segfaults instead)
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Explicit string length with SQL_NTS",
                 "[odbc-api][driverconnect][connecting]") {
  // Use SQL_NTS for connection string
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=NonExistentDSN"), SQL_NTS, nullptr, 0,
                                         nullptr, SQL_DRIVER_NOPROMPT);
  // Should fail with IM002 (data source not found)
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Explicit byte count for string length",
                 "[odbc-api][driverconnect][connecting]") {
  const std::string connStr = "DSN=NonExistentDSN";

  // Use explicit length
  const SQLRETURN ret =
      SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), static_cast<SQLSMALLINT>(connStr.length()),
                       nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Zero string length (empty string)",
                 "[odbc-api][driverconnect][connecting][edge]") {
  // Zero length string
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=test"),
                                         0,  // Zero length - treats as empty
                                         nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: SQL_DRIVER_NOPROMPT mode with complete DSN",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion]") {
  // SQL_DRIVER_NOPROMPT: Never prompt, fail if info missing
  // With complete DSN (has credentials), should succeed
  std::string connStr = "DSN=" + dsn_name();
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLDriverConnect: SQL_DRIVER_NOPROMPT mode with incomplete info fails",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion][error]") {
  // SQL_DRIVER_NOPROMPT: With DSN that has no credentials, should fail
  // Note: Snowflake driver returns 28000 for authentication failures.
  // ODBC spec allows 28000, 08001, 08004, or HY000, but Snowflake consistently uses 28000.
  std::string connStr = "DSN=" + dsn_name();
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_NOPROMPT);
  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: SQL_DRIVER_COMPLETE mode with complete DSN",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion]") {
  // SQL_DRIVER_COMPLETE: Prompt only if info is missing
  // With complete DSN, no prompting needed, should succeed
  std::string connStr = "DSN=" + dsn_name();
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_COMPLETE);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLDriverConnect: SQL_DRIVER_COMPLETE mode with incomplete info",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion][error]") {
  // SQL_DRIVER_COMPLETE: Would prompt for missing credentials,
  // but fails with NULL window handle in headless environment
  // Note: Snowflake driver returns 28000 for authentication failures.
  // ODBC spec allows 28000, 08001, 08004, or HY000, but Snowflake consistently uses 28000.
  std::string connStr = "DSN=" + dsn_name();
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_COMPLETE);
  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: SQL_DRIVER_COMPLETE_REQUIRED mode with complete DSN",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion]") {
  // SQL_DRIVER_COMPLETE_REQUIRED: Only prompt for required info
  // With complete DSN, no prompting needed, should succeed
  std::string connStr = "DSN=" + dsn_name();
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_COMPLETE_REQUIRED);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLDriverConnect: SQL_DRIVER_COMPLETE_REQUIRED mode with incomplete info",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion][error]") {
  // SQL_DRIVER_COMPLETE_REQUIRED: Would prompt for required credentials,
  // but fails with NULL window handle in headless environment
  // Note: Snowflake driver returns 28000 for authentication failures.
  // ODBC spec allows 28000, 08001, 08004, or HY000, but Snowflake consistently uses 28000.
  const std::string connStr = "DSN=" + dsn_name();
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_COMPLETE_REQUIRED);
  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: SQL_DRIVER_PROMPT mode always requires window handle",
                 "[odbc-api][driverconnect][dsn][integration][drivercompletion][error]") {
  // Given an allocated DBC handle and a complete DSN connection string
  const std::string connStr = "DSN=" + dsn_name();

  // When SQLDriverConnect is called with SQL_DRIVER_PROMPT and a NULL window handle
  //   (per ODBC spec, PROMPT must show a dialog, so a non-null hwnd is required)
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_PROMPT);

  WINDOWS_ONLY {
    // Then the DM rejects the call with HY024 (invalid argument value)
    REQUIRE_EXPECTED_ERROR(ret, "HY024", dbc_handle(), SQL_HANDLE_DBC);
  }
  UNIX_ONLY {
    NON_IODBC {
      // Then the DM follows spec and rejects with HY092 (invalid attribute id)
      REQUIRE_EXPECTED_ERROR(ret, "HY092", dbc_handle(), SQL_HANDLE_DBC);
    }
    IODBC_ONLY {
      // Then the iODBC DM also rejects SQL_DRIVER_PROMPT with a null window
      //   handle, but uses a DM-side error path that does not surface as
      //   HY092; only the return code (SQL_ERROR) is asserted here
      REQUIRE(ret == SQL_ERROR);
    }
  }
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: 08002 - Connection already in use",
                 "[odbc-api][driverconnect][dsn][integration][error]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());

  // First connection
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Second connection on same handle should fail with 08002
  ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                         SQL_DRIVER_NOPROMPT);
  REQUIRE_EXPECTED_ERROR(ret, "08002", dbc_handle(), SQL_HANDLE_DBC);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Connection string keyword=value parsing",
                 "[odbc-api][driverconnect][connecting][parsing]") {
  // Valid format but non-existent DSN
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=test;UID=user;PWD=pass"), SQL_NTS, nullptr,
                                         0, nullptr, SQL_DRIVER_NOPROMPT);
  // Should be IM002 (DSN not found), not a parsing error
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: OutConnectionString buffer handling",
                 "[odbc-api][driverconnect][dsn][integration][buffer]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());

  // Test with output buffer
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outConnStrLen = 0;
  std::memset(outConnStr, 0, sizeof(outConnStr));

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, outConnStr,
                                   sizeof(outConnStr), &outConnStrLen, SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Output connection string should be populated
  REQUIRE(outConnStrLen > 0);
  REQUIRE(outConnStr[0] != '\0');

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: OutConnectionString truncation (01004)",
                 "[odbc-api][driverconnect][dsn][integration][buffer]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());

  // Very small buffer to force truncation
  SQLCHAR outConnStr[10];
  SQLSMALLINT outConnStrLen = 0;

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, outConnStr,
                                   sizeof(outConnStr), &outConnStrLen, SQL_DRIVER_NOPROMPT);
  // Should succeed (connection made) but possibly with INFO for truncation
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // outConnStrLen should indicate the full length needed
  if (ret == SQL_SUCCESS_WITH_INFO) {
    const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
    IODBC_ONLY {
      // BD#61: the iODBC Driver Manager does not surface the driver's
      //   diagnostic record for SQLDriverConnect, so the truncation signal
      //   lives only in the SQL_SUCCESS_WITH_INFO return code. This holds for
      //   both the old and new driver (the new driver posts the 01004 record,
      //   but iODBC drops it before the application can read it).
      (void)records;
    }
    else {
      // May have 01004 (string truncated) or other info
      const bool found_truncation_or_other = !records.empty();
      REQUIRE(found_truncation_or_other);
    }
  }

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: NULL OutConnectionString with non-NULL length pointer",
                 "[odbc-api][driverconnect][dsn][integration][buffer]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());
  SQLSMALLINT outConnStrLen = 0;

  // NULL buffer but valid length pointer - should report required length
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, &outConnStrLen,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Length should indicate how much space is needed
  REQUIRE(outConnStrLen > 0);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: NULL StringLength2Ptr pointer",
                 "[odbc-api][driverconnect][dsn][integration][buffer]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());
  SQLCHAR outConnStr[1024];

  // Valid buffer but NULL length pointer - should still succeed
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, outConnStr,
                                   sizeof(outConnStr), nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Buffer should still be populated
  REQUIRE(outConnStr[0] != '\0');

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Both OutConnectionString and StringLength2Ptr NULL",
                 "[odbc-api][driverconnect][dsn][integration][buffer]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());

  // Both NULL - should still connect, just no output
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify connection works
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Connection string with special characters",
                 "[odbc-api][driverconnect][connecting][parsing]") {
  // Connection string with braces (should be parsed correctly)
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN={Test DSN With Spaces}"), SQL_NTS, nullptr,
                                         0, nullptr, SQL_DRIVER_NOPROMPT);
  // Will fail with IM002 but shouldn't crash or have parsing error
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Multiple semicolons in connection string",
                 "[odbc-api][driverconnect][connecting][parsing]") {
  // Multiple semicolons should be handled gracefully
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("DSN=test;;UID=user;;;"), SQL_NTS, nullptr, 0,
                                         nullptr, SQL_DRIVER_NOPROMPT);
  // Should parse correctly and fail with IM002
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE_METHOD(DbcFixture, "SQLDriverConnect: Case insensitivity of keywords",
                 "[odbc-api][driverconnect][connecting][parsing]") {
  // Keywords are case-insensitive per ODBC spec
  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar("dsn=test;uid=user;pwd=pass"), SQL_NTS, nullptr,
                                         0, nullptr, SQL_DRIVER_NOPROMPT);
  // Should parse correctly (same as DSN=test) - DSN not found, not parsing error
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLDriverConnect - DSN-Based Integration Tests
// ============================================================================
// These tests require a configured DSN. They will FAIL if DSN is not set up.

TEST_CASE("SQLDriverConnect: DSN configuration check", "[odbc-api][driverconnect][dsn][integration][!mayfail]") {
  const auto config = DataSourceConfig::Snowflake().install();

  const std::string dsn = config.dsn_name();
  // DSN name has unique suffix for parallel test isolation
  REQUIRE(dsn.rfind("Snowflake_", 0) == 0);  // C++17 compatible starts_with
  REQUIRE(dsn.length() > 10);                // Has suffix
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Basic DSN connection succeeds",
                 "[odbc-api][driverconnect][dsn][integration]") {
  // A bare "DSN=<name>" connection string carries no inline parameters, so the
  //   driver must expand every attribute stored in the DSN entry (registry /
  //   odbc.ini) — account, host, user, credentials — before it can connect.
  const std::string connStr = build_dsn_connection_string(dsn_name());

  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_NOPROMPT);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  // Verify the session by round-tripping a query. The statement is scoped so
  //   its handle is freed before SQLDisconnect: disconnecting a connection
  //   implicitly frees its statements, so freeing the wrapper afterwards would
  //   double-free an already-released handle.
  {
    StatementHandleWrapper stmt(dbc_handle(), SQL_HANDLE_STMT);
    CHECK_THAT(
        OdbcResult(SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS), SQL_HANDLE_STMT, stmt.getHandle()),
        OdbcMatchers::Succeeded());
    CHECK_THAT(OdbcResult(SQLFetch(stmt.getHandle()), SQL_HANDLE_STMT, stmt.getHandle()), OdbcMatchers::Succeeded());
  }

  REQUIRE(SQLDisconnect(dbc_handle()) == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Connection with additional parameters",
                 "[odbc-api][driverconnect][dsn][integration]") {
  // DSN with additional Snowflake-specific parameters
  const std::string connStr = "DSN=" + dsn_name() + ";TRACING=0";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLDriverConnect: Unrecognized connection string keyword (old driver: 01S00, new driver: no warning)",
                 "[odbc-api][driverconnect][dsn][integration]") {
  const std::string connStr = "DSN=" + dsn_name() + ";INVALIDKEY=abc";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  auto has_01S00 = [&records]() {
    for (const auto& r : records) {
      if (r.sqlState == "01S00") return true;
    }
    return false;
  };

  // The unrecognized keyword does not prevent the connection: the old driver's 01S00 and
  //   the new driver's 01004 OutConnectionString info are both SQL_SUCCESS_WITH_INFO, not
  //   errors, so the connection opens on both.
  REQUIRE(SQL_SUCCEEDED(ret));

  NEW_DRIVER_ONLY("BD#106") {
    // BD#106: the new driver forwards unrecognized connection-string keywords to the
    //   server as session parameters rather than rejecting them locally, so it does not
    //   post the 01S00 warning. SNOW-3831223 will source 01S00 from a server-side
    //   "parameter not applied" signal, reaching parity with the old driver.
    REQUIRE_FALSE(has_01S00());
  }
  OLD_DRIVER_ONLY("BD#106") {
    OLD_IODBC_ONLY("BD#61") {
      // The old driver under iODBC reports SQL_SUCCESS_WITH_INFO for the unrecognized
      //   keyword but doesn't post the 01S00 diagnostic record on the DBC handle.
      (void)records;
    }
    else {
      // Per ODBC spec, the old driver flags the unrecognized keyword with 01S00.
      REQUIRE(has_01S00());
    }
  }

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLDriverConnect: 28000 - Invalid authorization specification",
                 "[odbc-api][driverconnect][dsn][integration][error]") {
  // Use DSN without auth but provide invalid credentials
  // Note: Snowflake driver returns 28000 for authentication failures.
  // ODBC spec allows 28000, 08001, 08004, or HY000, but Snowflake consistently uses 28000.
  const std::string connStr = "DSN=" + dsn_name() + ";UID=invalid_user_xyz;PWD=invalid_cred_xyz";

  const SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                         SQL_DRIVER_NOPROMPT);
  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Disconnect and reconnect cycle",
                 "[odbc-api][driverconnect][dsn][integration][lifecycle]") {
  const std::string connStr = build_dsn_connection_string(dsn_name());

  for (int i = 0; i < 3; i++) {
    SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                     SQL_DRIVER_NOPROMPT);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

    // Verify connection with query
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLDisconnect(dbc_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
}

TEST_CASE_METHOD(EnvDefaultDSNFixture, "SQLDriverConnect: Multiple concurrent connections",
                 "[odbc-api][driverconnect][dsn][integration][concurrent]") {
  constexpr int NUM_CONNECTIONS = 4;
  SQLHDBC connections[NUM_CONNECTIONS];

  const std::string connStr = build_dsn_connection_string(dsn_name());
  int successful_connections = 0;

  // Allocate and connect all handles
  for (auto& connection : connections) {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &connection);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLDriverConnect(connection, nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                           SQL_DRIVER_NOPROMPT);
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      successful_connections++;
    }
  }

  REQUIRE(successful_connections == NUM_CONNECTIONS);

  // Cleanup all connections
  for (auto& connection : connections) {
    SQLRETURN ret = SQLDisconnect(connection);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLFreeHandle(SQL_HANDLE_DBC, connection);
    REQUIRE(ret == SQL_SUCCESS);
  }
}

// ============================================================================
// SQLDriverConnect - Snowflake-Specific Connection String Parameters
// ============================================================================
// Note: These tests document and verify Snowflake ODBC driver-specific parameters
// that are not part of the standard ODBC specification. These parameters are
// extensions provided by the Snowflake driver for driver-specific functionality.

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake TRACING parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // Note: TRACING is a Snowflake-specific parameter that controls logging level (0-6)
  const std::string connStr = "DSN=" + dsn_name() + ";TRACING=0";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake CLIENT_SESSION_KEEP_ALIVE parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // Note: CLIENT_SESSION_KEEP_ALIVE is a Snowflake-specific parameter that enables heartbeat to keep session alive
  const std::string connStr = "DSN=" + dsn_name() + ";CLIENT_SESSION_KEEP_ALIVE=true";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify connection is usable
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake LOGIN_TIMEOUT parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // LOGIN_TIMEOUT sets connection timeout in seconds
  const std::string connStr = "DSN=" + dsn_name() + ";LOGIN_TIMEOUT=120";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake APPLICATION parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // APPLICATION parameter sets client application name
  const std::string connStr = "DSN=" + dsn_name() + ";APPLICATION=ODBCTestSuite";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify connection is usable
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake QUERY_TIMEOUT parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // QUERY_TIMEOUT sets query execution timeout (0 = no timeout)
  const std::string connStr = "DSN=" + dsn_name() + ";QUERY_TIMEOUT=0";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake NETWORK_TIMEOUT parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // NETWORK_TIMEOUT sets network operation timeout
  const std::string connStr = "DSN=" + dsn_name() + ";NETWORK_TIMEOUT=0";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake DisableOCSPCheck parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake][security]") {
  // Note: DisableOCSPCheck is a Snowflake-specific parameter that controls OCSP certificate validation
  const std::string connStr = "DSN=" + dsn_name() + ";DisableOCSPCheck=true";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake DATABASE and SCHEMA parameters",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // DATABASE and SCHEMA can override default namespace
  // Empty values should not break connection
  const std::string connStr = "DSN=" + dsn_name() + ";DATABASE=;SCHEMA=";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake WAREHOUSE parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // WAREHOUSE can override default warehouse (empty = use default)
  const std::string connStr = "DSN=" + dsn_name() + ";WAREHOUSE=";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake ROLE parameter",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // ROLE can override default role (empty = use default)
  const std::string connStr = "DSN=" + dsn_name() + ";ROLE=";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDriverConnect: Snowflake multiple parameters combined",
                 "[odbc-api][driverconnect][dsn][integration][snowflake]") {
  // Combination of multiple Snowflake-specific parameters
  const std::string connStr = "DSN=" + dsn_name() +
                              ";APPLICATION=ODBCTest"
                              ";TRACING=0"
                              ";LOGIN_TIMEOUT=60"
                              ";QUERY_TIMEOUT=0"
                              ";NETWORK_TIMEOUT=0"
                              ";CLIENT_SESSION_KEEP_ALIVE=false";

  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify connection works
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecDirect(stmt, sqlchar("SELECT CURRENT_TIMESTAMP()"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLDriverConnect - Threaded/Concurrent Tests
// ============================================================================

static void driver_connect_thread(const std::string& connStr, std::atomic<SQLRETURN>* result, const int iterations) {
  for (int i = 0; i < iterations; i++) {
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

    ret = SQLDriverConnect(dbc, nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      // Verify with simple query
      SQLHSTMT stmt = SQL_NULL_HSTMT;
      if (SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt) == SQL_SUCCESS) {
        SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
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

// [flaky]: this drives NUM_THREADS concurrent SQLDriverConnect calls against the
// reference (old) driver, which is not reliably thread-safe (intermittent
// simba_abort / pthread_mutex_lock aborts — see async_execution.cpp). It passes
// most runs but fails intermittently on the reference matrix (macOS + iODBC).
// Tagged flaky to unblock the reference job; fix tracked as follow-up.
TEST_CASE("SQLDriverConnect: Threaded concurrent connections",
          "[odbc-api][driverconnect][dsn][integration][concurrent][threads][flaky]") {
  const auto config = DataSourceConfig::Snowflake().install();

  constexpr int NUM_THREADS = 4;

  const std::string connStr = build_dsn_connection_string(config.dsn_name());

  // Pre-connect to ensure driver is loaded
  {
    SQLHENV env = SQL_NULL_HENV;
    SQLHDBC dbc = SQL_NULL_HDBC;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLDriverConnect(dbc, nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
  }

  std::atomic<SQLRETURN> results[NUM_THREADS];
  std::vector<std::thread> threads;

  for (std::atomic<SQLRETURN>& result : results) {
    constexpr int ITERATIONS_PER_THREAD = 2;
    result.store(SQL_SUCCESS, std::memory_order_relaxed);
    threads.emplace_back(driver_connect_thread, connStr, &result, ITERATIONS_PER_THREAD);
  }

  for (auto& t : threads) {
    t.join();
  }

  int successful = 0;
  for (const std::atomic<SQLRETURN>& result : results) {
    const SQLRETURN value = result.load(std::memory_order_acquire);
    if (value == SQL_SUCCESS) {
      successful++;
    }
  }
  REQUIRE(successful == NUM_THREADS);
}

// ============================================================================
// SQLDriverConnect - Connection Pooling Repro
// ============================================================================
// Reproduces, outside the replay harness, the connect setup used by the Excel
// Power Query replay `excel/powerquery/raw_sql/all_datatypes`, which fails its
// first SQLDriverConnect with SQL_ERROR (and, on the Windows DM, an *empty*
// diagnostic record) on the new driver. That replay and its sibling
// `raw_sql/select_1` share byte-identical connect code, yet only the former
// fails — the distinguishing factor versus every other case in this file is
// process-wide connection pooling (SQL_ATTR_CONNECTION_POOLING =
// SQL_CP_ONE_PER_HENV) combined with a full connection string (not DSN=) and a
// short login timeout.
//
// This test mirrors that exact sequence: two pooling-enabled environments, a
// DBC allocated on the second, SQL_ATTR_LOGIN_TIMEOUT, then a connect via the
// full connection string. It intentionally runs on BOTH drivers (no
// SKIP_NEW_DRIVER_NOT_IMPLEMENTED) so CI confirms whether the failure
// reproduces here; on failure the matcher prints the return code and any
// diagnostic records, closing the "empty diagnostic" blind spot.
TEST_CASE("SQLDriverConnect: connection pooling repro (Excel all_datatypes)",
          "[odbc-api][driverconnect][dsn][integration][pooling][repro]") {
  const auto config = DataSourceConfig::Snowflake().install();
  const std::string connStr = config.connection_string();

  // First pooling-enabled environment (mirrors the captured trace's env0).
  SQLHENV env0 = SQL_NULL_HENV;
  REQUIRE(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0) == SQL_SUCCESS);
  REQUIRE(SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0) == SQL_SUCCESS);
  CHECK_THAT(
      OdbcResult(SQLSetEnvAttr(env0, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_ONE_PER_HENV), 0),
                 SQL_HANDLE_ENV, env0),
      OdbcMatchers::IsSuccess());

  // Second pooling-enabled environment; the connection is allocated here (env1).
  SQLHENV env1 = SQL_NULL_HENV;
  REQUIRE(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1) == SQL_SUCCESS);
  REQUIRE(SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0) == SQL_SUCCESS);
  CHECK_THAT(
      OdbcResult(SQLSetEnvAttr(env1, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_ONE_PER_HENV), 0),
                 SQL_HANDLE_ENV, env1),
      OdbcMatchers::IsSuccess());

  SQLHDBC dbc = SQL_NULL_HDBC;
  REQUIRE_THAT(OdbcResult(SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc), SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  REQUIRE(dbc != SQL_NULL_HDBC);

  SQLRETURN ret = SQLSetConnectAttr(dbc, SQL_ATTR_LOGIN_TIMEOUT, reinterpret_cast<SQLPOINTER>(15), 0);
  CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc), OdbcMatchers::IsSuccess());

  ret = SQLDriverConnect(dbc, nullptr, sqlchar(connStr.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc), OdbcMatchers::Succeeded());

  if (SQL_SUCCEEDED(ret)) {
    SQLDisconnect(dbc);
  }
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env1);
  SQLFreeHandle(SQL_HANDLE_ENV, env0);
}
