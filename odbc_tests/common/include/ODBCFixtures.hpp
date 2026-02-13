#ifndef ODBCFIXTURES_HPP
#define ODBCFIXTURES_HPP

#include <sql.h>
#include <sqlext.h>
#include <optional>

#include <catch2/catch_test_macros.hpp>
#include <utility>
#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"

// ============================================================================
// Base Fixtures (Parameterized via Constructor)
// ============================================================================

class EnvFixture {
public:
  std::optional<ConfigInstallation> config;
  std::optional<EnvironmentHandleWrapper> env_wrapper;
  SQLHENV env = SQL_NULL_HENV;

  // Constructor with optional DSN configuration
  explicit EnvFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt) {
    // Install DSN BEFORE creating ENV handle (critical for UnixODBC caching)
    if (dsn_config.has_value()) {
      config = dsn_config->install();
    }

    // Create ENV handle (will see installed DSN)
    env_wrapper.emplace();
    env = env_wrapper->getHandle();
    SQLRETURN ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    REQUIRE(ret == SQL_SUCCESS);
  }

  // Disable copy and move (RAII resource management)
  EnvFixture(const EnvFixture&) = delete;
  EnvFixture& operator=(const EnvFixture&) = delete;
  EnvFixture(EnvFixture&&) = delete;
  EnvFixture& operator=(EnvFixture&&) = delete;

  SQLHENV env_handle() const { return env; }
};

class DbcFixture : public EnvFixture {
public:
  std::optional<ConnectionHandleWrapper> dbc_wrapper;
  SQLHDBC dbc = SQL_NULL_HDBC;

  // Constructor with optional DSN configuration
  explicit DbcFixture(std::optional<DataSourceConfig> dsn_config = std::nullopt)
    : EnvFixture(std::move(dsn_config)) {
    dbc_wrapper.emplace(env_wrapper->createConnectionHandle());
    dbc = dbc_wrapper->getHandle();
  }

  // Disable copy and move (RAII resource management)
  DbcFixture(const DbcFixture&) = delete;
  DbcFixture& operator=(const DbcFixture&) = delete;
  DbcFixture(DbcFixture&&) = delete;
  DbcFixture& operator=(DbcFixture&&) = delete;

  SQLHDBC dbc_handle() const { return dbc; }
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

#endif // ODBCFIXTURES_HPP
