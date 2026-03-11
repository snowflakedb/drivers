#ifndef ODBCFIXTURES_HPP
#define ODBCFIXTURES_HPP

#include <sql.h>
#include <sqlext.h>

#include <optional>
#include <string>
#include <utility>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"

// ============================================================================
// Base Fixtures (Parameterized via Constructor)
// ============================================================================

class EnvFixture {
  std::optional<ConfigInstallation> config;
  std::optional<EnvironmentHandleWrapper> env_wrapper;

 public:
  // Constructor with optional DSN configuration
  explicit EnvFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt) {
    // Install DSN BEFORE creating ENV handle (critical for UnixODBC caching)
    if (dsn_config.has_value()) {
      config = dsn_config->install();
    }

    // Create ENV handle (will see installed DSN)
    env_wrapper.emplace();
    SQLRETURN ret =
        SQLSetEnvAttr(env_wrapper->getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    REQUIRE(ret == SQL_SUCCESS);
  }

  // Disable copy and move (RAII resource management)
  EnvFixture(const EnvFixture&) = delete;
  EnvFixture& operator=(const EnvFixture&) = delete;
  EnvFixture(EnvFixture&&) = delete;
  EnvFixture& operator=(EnvFixture&&) = delete;

  [[nodiscard]] SQLHENV env_handle() const { return env_wrapper->getHandle(); }
  [[nodiscard]] std::string dsn_name() const { return config.value().dsn_name(); }
  [[nodiscard]] std::string connection_string() const { return config.value().connection_string(); }

 protected:
  [[nodiscard]] ConnectionHandleWrapper create_connection_handle() { return env_wrapper->createConnectionHandle(); }
};

class DbcFixture : public EnvFixture {
  std::optional<ConnectionHandleWrapper> dbc_wrapper;

 public:
  // Constructor with optional DSN configuration
  explicit DbcFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt) : EnvFixture(std::move(dsn_config)) {
    dbc_wrapper.emplace(create_connection_handle());
  }

  // Disable copy and move (RAII resource management)
  DbcFixture(const DbcFixture&) = delete;
  DbcFixture& operator=(const DbcFixture&) = delete;
  DbcFixture(DbcFixture&&) = delete;
  DbcFixture& operator=(DbcFixture&&) = delete;

  [[nodiscard]] SQLHDBC dbc_handle() const { return dbc_wrapper->getHandle(); }

  // Releases the connection handle wrapper, preventing double-free when test
  // code has already freed the underlying HDBC via SQLFreeHandle.
  void release_dbc() { dbc_wrapper.reset(); }
};

// ============================================================================
// Convenience Fixtures for TEST_CASE_METHOD (Pre-configured DSNs)
// ============================================================================

class EnvDefaultDSNFixture : public EnvFixture {
 public:
  EnvDefaultDSNFixture() : EnvFixture(DataSourceConfig::Snowflake()) {}
};

class DbcDefaultDSNFixture : public DbcFixture {
 public:
  DbcDefaultDSNFixture() : DbcFixture(DataSourceConfig::Snowflake()) {}
};

class EnvNoAuthDSNFixture : public EnvFixture {
 public:
  EnvNoAuthDSNFixture() : EnvFixture(DataSourceConfig::SnowflakeNoAuth()) {}
};

class DbcNoAuthDSNFixture : public DbcFixture {
 public:
  DbcNoAuthDSNFixture() : DbcFixture(DataSourceConfig::SnowflakeNoAuth()) {}
};

// ============================================================================
// Connected Statement Fixtures (ENV + DBC + SQLConnect + STMT)
// ============================================================================

class StmtFixture : public DbcFixture {
  SQLHSTMT stmt = SQL_NULL_HSTMT;

 public:
  explicit StmtFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt) : DbcFixture(std::move(dsn_config)) {
    // SQLConnect is not yet implemented in the new driver
    SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

    const std::string dsn = dsn_name();
    SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn.c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
    REQUIRE(ret == SQL_SUCCESS);
  }

  ~StmtFixture() {
    if (stmt != SQL_NULL_HSTMT) {
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    }
    SQLDisconnect(dbc_handle());
  }

  // Disable copy and move (RAII resource management)
  StmtFixture(const StmtFixture&) = delete;
  StmtFixture& operator=(const StmtFixture&) = delete;
  StmtFixture(StmtFixture&&) = delete;
  StmtFixture& operator=(StmtFixture&&) = delete;

  [[nodiscard]] SQLHSTMT stmt_handle() const { return stmt; }
};

class StmtDefaultDSNFixture : public StmtFixture {
 public:
  StmtDefaultDSNFixture() : StmtFixture(DataSourceConfig::Snowflake()) {}
};

// ============================================================================
// Two-Statement Fixture (same connection, two independent statement handles)
// ============================================================================

class TwoStmtFixture : public StmtFixture {
  SQLHSTMT stmt2 = SQL_NULL_HSTMT;

 public:
  explicit TwoStmtFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt)
      : StmtFixture(std::move(dsn_config)) {
    const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
    REQUIRE(ret == SQL_SUCCESS);
  }

  ~TwoStmtFixture() {
    if (stmt2 != SQL_NULL_HSTMT) {
      SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
    }
  }

  // Disable copy and move (RAII resource management)
  TwoStmtFixture(const TwoStmtFixture&) = delete;
  TwoStmtFixture& operator=(const TwoStmtFixture&) = delete;
  TwoStmtFixture(TwoStmtFixture&&) = delete;
  TwoStmtFixture& operator=(TwoStmtFixture&&) = delete;

  [[nodiscard]] SQLHSTMT stmt2_handle() const { return stmt2; }
};

class TwoStmtDefaultDSNFixture : public TwoStmtFixture {
 public:
  TwoStmtDefaultDSNFixture() : TwoStmtFixture(DataSourceConfig::Snowflake()) {}
};

#endif  // ODBCFIXTURES_HPP
