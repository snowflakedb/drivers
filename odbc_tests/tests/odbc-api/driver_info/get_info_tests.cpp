#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include <cstring>
#include <string>

#include "compatibility.hpp"
#include "ODBCFixtures.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"

// ============================================================================
// Driver Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ACTIVE_ENVIRONMENTS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT activeEnv = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ACTIVE_ENVIRONMENTS, &activeEnv, sizeof(activeEnv), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(activeEnv == 0); // no limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ASYNC_DBC_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER asyncDbc = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ASYNC_DBC_FUNCTIONS, &asyncDbc, sizeof(asyncDbc), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(asyncDbc == SQL_ASYNC_DBC_CAPABLE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ASYNC_MODE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER asyncMode = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ASYNC_MODE, &asyncMode, sizeof(asyncMode), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(asyncMode == SQL_AM_STATEMENT); // Statement-level async support

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ASYNC_NOTIFICATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER asyncNotif = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ASYNC_NOTIFICATION, &asyncNotif, sizeof(asyncNotif), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(asyncNotif == SQL_ASYNC_NOTIFICATION_NOT_CAPABLE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_BATCH_ROW_COUNT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER batchRowCount = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_BATCH_ROW_COUNT, &batchRowCount, sizeof(batchRowCount), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(batchRowCount == 0); // Snowflake doesn't support batch row counts

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_BATCH_SUPPORT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER batchSupport = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_BATCH_SUPPORT, &batchSupport, sizeof(batchSupport), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(batchSupport == 0); // Snowflake doesn't support batch statements

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DATA_SOURCE_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char dsnName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DATA_SOURCE_NAME, dsnName, sizeof(dsnName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);
  REQUIRE(std::string(dsnName) == config.value().dsn_name());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DRIVER_AWARE_POOLING_SUPPORTED",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER poolingSupport = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_AWARE_POOLING_SUPPORTED, &poolingSupport, sizeof(poolingSupport), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(poolingSupport == SQL_DRIVER_AWARE_POOLING_NOT_CAPABLE);

  SQLDisconnect(dbc_handle());
}

// SQL_DRIVER_HDBC, SQL_DRIVER_HDESC, SQL_DRIVER_HENV, SQL_DRIVER_HLIB & SQL_DRIVER_HSTMT
// are implemented by the driver manager alone

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DRIVER_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char driverName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, driverName, sizeof(driverName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);
  REQUIRE(std::string(driverName) == "Snowflake");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DRIVER_ODBC_VER",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char odbcVer[256];
  SQLSMALLINT verLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_ODBC_VER, odbcVer, sizeof(odbcVer), &verLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(verLen > 0);
  REQUIRE(std::string(odbcVer).substr(0, 2) == "03"); // ODBC 3.x

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DRIVER_VER",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char driverVer[256];
  SQLSMALLINT verLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_VER, driverVer, sizeof(driverVer), &verLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(verLen > 0);
  REQUIRE(std::count(driverVer, driverVer + verLen, '.') == 2); // Format: ##.##.####

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DYNAMIC_CURSOR_ATTRIBUTES1",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DYNAMIC_CURSOR_ATTRIBUTES1, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support dynamic cursors
  REQUIRE((attrs & SQL_CA1_NEXT) == 0);
  REQUIRE((attrs & SQL_CA1_ABSOLUTE) == 0);
  REQUIRE((attrs & SQL_CA1_RELATIVE) == 0);
  REQUIRE((attrs & SQL_CA1_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_EXCLUSIVE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_NO_CHANGE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_UNLOCK) == 0);
  REQUIRE((attrs & SQL_CA1_POS_POSITION) == 0);
  REQUIRE((attrs & SQL_CA1_POS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_REFRESH) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_SELECT_FOR_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_ADD) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_UPDATE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_DELETE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_FETCH_BY_BOOKMARK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DYNAMIC_CURSOR_ATTRIBUTES2",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DYNAMIC_CURSOR_ATTRIBUTES2, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support dynamic cursors
  REQUIRE((attrs & SQL_CA2_READ_ONLY_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_LOCK_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_ROWVER_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_VALUES_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_ADDITIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_DELETIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_UPDATES) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_SELECT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_INSERT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_CATALOG) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_AFFECTS_ALL) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_EXACT) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_APPROXIMATE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_NON_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_TRY_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_UNIQUE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES1, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((attrs & SQL_CA1_NEXT) == SQL_CA1_NEXT); // Reference driver supports NEXT for forward-only cursors
  REQUIRE((attrs & SQL_CA1_LOCK_EXCLUSIVE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_NO_CHANGE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_UNLOCK) == 0);
  REQUIRE((attrs & SQL_CA1_POS_POSITION) == 0);
  REQUIRE((attrs & SQL_CA1_POS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_REFRESH) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_SELECT_FOR_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_ADD) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_UPDATE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_DELETE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_FETCH_BY_BOOKMARK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES2",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_FORWARD_ONLY_CURSOR_ATTRIBUTES2, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support advanced forward-only cursor attributes
  REQUIRE((attrs & SQL_CA2_READ_ONLY_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_LOCK_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_ROWVER_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_VALUES_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_ADDITIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_DELETIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_UPDATES) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_SELECT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_INSERT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_CATALOG) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_AFFECTS_ALL) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_EXACT) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_APPROXIMATE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_NON_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_TRY_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_UNIQUE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_FILE_USAGE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT fileUsage = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_FILE_USAGE, &fileUsage, sizeof(fileUsage), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(fileUsage == SQL_FILE_NOT_SUPPORTED); // Two-tier driver

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_GETDATA_EXTENSIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER getdataExt = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_GETDATA_EXTENSIONS, &getdataExt, sizeof(getdataExt), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((getdataExt & SQL_GD_ANY_COLUMN) == SQL_GD_ANY_COLUMN); // Reference driver supports SQLGetData for any unbound column
  REQUIRE((getdataExt & SQL_GD_ANY_ORDER) == SQL_GD_ANY_ORDER); // Reference driver supports SQLGetData for unbound columns in any order
  REQUIRE((getdataExt & SQL_GD_BLOCK) == 0);
  REQUIRE((getdataExt & SQL_GD_BOUND) == SQL_GD_BOUND); // Reference driver supports SQLGetData for bound columns
  REQUIRE((getdataExt & SQL_GD_OUTPUT_PARAMS) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_INFO_SCHEMA_VIEWS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER infoSchemaViews = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_INFO_SCHEMA_VIEWS, &infoSchemaViews, sizeof(infoSchemaViews), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE((infoSchemaViews & SQL_ISV_ASSERTIONS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_CHARACTER_SETS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_CHECK_CONSTRAINTS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_COLLATIONS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_COLUMN_DOMAIN_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_COLUMN_PRIVILEGES) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_COLUMNS) == SQL_ISV_COLUMNS); // Reference driver supports COLUMNS view
  REQUIRE((infoSchemaViews & SQL_ISV_CONSTRAINT_COLUMN_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_CONSTRAINT_TABLE_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_DOMAIN_CONSTRAINTS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_DOMAINS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_KEY_COLUMN_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_REFERENTIAL_CONSTRAINTS) == SQL_ISV_REFERENTIAL_CONSTRAINTS); // Reference driver supports REFERENTIAL_CONSTRAINTS view
  REQUIRE((infoSchemaViews & SQL_ISV_SCHEMATA) == SQL_ISV_SCHEMATA); // Reference driver supports SCHEMATA view
  REQUIRE((infoSchemaViews & SQL_ISV_SQL_LANGUAGES) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_TABLE_CONSTRAINTS) == SQL_ISV_TABLE_CONSTRAINTS); // Reference driver supports TABLE_CONSTRAINTS view
  REQUIRE((infoSchemaViews & SQL_ISV_TABLE_PRIVILEGES) == SQL_ISV_TABLE_PRIVILEGES); // Reference driver supports TABLE_PRIVILEGES view
  REQUIRE((infoSchemaViews & SQL_ISV_TABLES) == SQL_ISV_TABLES); // Reference driver supports TABLES view
  REQUIRE((infoSchemaViews & SQL_ISV_TRANSLATIONS) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_USAGE_PRIVILEGES) == SQL_ISV_USAGE_PRIVILEGES); // Reference driver supports USAGE_PRIVILEGES view
  REQUIRE((infoSchemaViews & SQL_ISV_VIEW_COLUMN_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_VIEW_TABLE_USAGE) == 0);
  REQUIRE((infoSchemaViews & SQL_ISV_VIEWS) == SQL_ISV_VIEWS); // Reference driver supports VIEWS view

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_KEYSET_CURSOR_ATTRIBUTES1",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_KEYSET_CURSOR_ATTRIBUTES1, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support keyset-driven cursors
  REQUIRE((attrs & SQL_CA1_NEXT) == 0);
  REQUIRE((attrs & SQL_CA1_ABSOLUTE) == 0);
  REQUIRE((attrs & SQL_CA1_RELATIVE) == 0);
  REQUIRE((attrs & SQL_CA1_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_EXCLUSIVE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_NO_CHANGE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_UNLOCK) == 0);
  REQUIRE((attrs & SQL_CA1_POS_POSITION) == 0);
  REQUIRE((attrs & SQL_CA1_POS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_REFRESH) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_SELECT_FOR_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_ADD) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_UPDATE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_DELETE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_FETCH_BY_BOOKMARK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_KEYSET_CURSOR_ATTRIBUTES2",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_KEYSET_CURSOR_ATTRIBUTES2, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support keyset-driven cursors
  REQUIRE((attrs & SQL_CA2_READ_ONLY_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_LOCK_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_ROWVER_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_VALUES_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_ADDITIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_DELETIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_UPDATES) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_SELECT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_INSERT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_CATALOG) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_AFFECTS_ALL) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_EXACT) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_APPROXIMATE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_NON_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_TRY_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_UNIQUE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_ASYNC_CONCURRENT_STATEMENTS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxAsync = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_ASYNC_CONCURRENT_STATEMENTS, &maxAsync, sizeof(maxAsync), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxAsync == 0); // No limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_CONCURRENT_ACTIVITIES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxConcurrent = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_CONCURRENT_ACTIVITIES, &maxConcurrent, sizeof(maxConcurrent), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxConcurrent == 0); // No limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_DRIVER_CONNECTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxConnections = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_DRIVER_CONNECTIONS, &maxConnections, sizeof(maxConnections), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxConnections == 0); // No limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ODBC_INTERFACE_CONFORMANCE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER interfaceConformance = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ODBC_INTERFACE_CONFORMANCE, &interfaceConformance, sizeof(interfaceConformance), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(interfaceConformance == SQL_OIC_CORE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ODBC_VER",
                 "[odbc-api][getinfo][driver_info]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char odbcVer[256];
  SQLSMALLINT verLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ODBC_VER, odbcVer, sizeof(odbcVer), &verLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(verLen > 0);
  REQUIRE(std::string(odbcVer).substr(0, 3) == "03."); // ODBC 3.x

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_PARAM_ARRAY_ROW_COUNTS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER paramArrayRowCounts = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_PARAM_ARRAY_ROW_COUNTS, &paramArrayRowCounts, sizeof(paramArrayRowCounts), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(paramArrayRowCounts == SQL_PARC_NO_BATCH);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_PARAM_ARRAY_SELECTS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER paramArraySelects = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_PARAM_ARRAY_SELECTS, &paramArraySelects, sizeof(paramArraySelects), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(paramArraySelects == SQL_PAS_NO_BATCH);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ROW_UPDATES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char rowUpdates[8];
  ret = SQLGetInfo(dbc_handle(), SQL_ROW_UPDATES, rowUpdates, sizeof(rowUpdates), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(rowUpdates) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SEARCH_PATTERN_ESCAPE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char escapeChar[8];
  SQLSMALLINT escapeLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SEARCH_PATTERN_ESCAPE, escapeChar, sizeof(escapeChar), &escapeLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(escapeChar) == "\\"); // Backslash escape

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SERVER_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char serverName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SERVER_NAME, serverName, sizeof(serverName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_STATIC_CURSOR_ATTRIBUTES1",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_STATIC_CURSOR_ATTRIBUTES1, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support static cursors
  REQUIRE((attrs & SQL_CA1_NEXT) == 0);
  REQUIRE((attrs & SQL_CA1_ABSOLUTE) == 0);
  REQUIRE((attrs & SQL_CA1_RELATIVE) == 0);
  REQUIRE((attrs & SQL_CA1_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_EXCLUSIVE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_NO_CHANGE) == 0);
  REQUIRE((attrs & SQL_CA1_LOCK_UNLOCK) == 0);
  REQUIRE((attrs & SQL_CA1_POS_POSITION) == 0);
  REQUIRE((attrs & SQL_CA1_POS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_POS_REFRESH) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_POSITIONED_DELETE) == 0);
  REQUIRE((attrs & SQL_CA1_SELECT_FOR_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_ADD) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_UPDATE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_DELETE_BY_BOOKMARK) == 0);
  REQUIRE((attrs & SQL_CA1_BULK_FETCH_BY_BOOKMARK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_STATIC_CURSOR_ATTRIBUTES2",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER attrs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_STATIC_CURSOR_ATTRIBUTES2, &attrs, sizeof(attrs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support static cursors
  REQUIRE((attrs & SQL_CA2_READ_ONLY_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_LOCK_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_ROWVER_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_OPT_VALUES_CONCURRENCY) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_ADDITIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_DELETIONS) == 0);
  REQUIRE((attrs & SQL_CA2_SENSITIVITY_UPDATES) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_SELECT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_INSERT) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_DELETE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_UPDATE) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_CATALOG) == 0);
  REQUIRE((attrs & SQL_CA2_MAX_ROWS_AFFECTS_ALL) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_EXACT) == 0);
  REQUIRE((attrs & SQL_CA2_CRC_APPROXIMATE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_NON_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_TRY_UNIQUE) == 0);
  REQUIRE((attrs & SQL_CA2_SIMULATE_UNIQUE) == 0);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// DBMS Product Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DATABASE_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char dbName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DATABASE_NAME, dbName, sizeof(dbName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);
  REQUIRE(!std::string(dbName).empty());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DBMS_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char dbmsName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DBMS_NAME, dbmsName, sizeof(dbmsName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);
  REQUIRE(std::string(dbmsName) == "Snowflake");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DBMS_VER",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char dbmsVer[256];
  SQLSMALLINT verLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DBMS_VER, dbmsVer, sizeof(dbmsVer), &verLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(verLen > 0);
  REQUIRE(std::count(dbmsVer, dbmsVer + verLen, '.') == 2); // Format: ##.##.####

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Data Source Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ACCESSIBLE_PROCEDURES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char accessible[8];
  ret = SQLGetInfo(dbc_handle(), SQL_ACCESSIBLE_PROCEDURES, accessible, sizeof(accessible), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(accessible) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ACCESSIBLE_TABLES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char accessible[8];
  ret = SQLGetInfo(dbc_handle(), SQL_ACCESSIBLE_TABLES, accessible, sizeof(accessible), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(accessible) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_BOOKMARK_PERSISTENCE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER bookmarkPersist = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_BOOKMARK_PERSISTENCE, &bookmarkPersist, sizeof(bookmarkPersist), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support bookmark persistence
  REQUIRE((bookmarkPersist & SQL_BP_CLOSE) == 0);
  REQUIRE((bookmarkPersist & SQL_BP_DELETE) == 0);
  REQUIRE((bookmarkPersist & SQL_BP_DROP) == 0);
  REQUIRE((bookmarkPersist & SQL_BP_TRANSACTION) == 0);
  REQUIRE((bookmarkPersist & SQL_BP_UPDATE) == 0);
  REQUIRE((bookmarkPersist & SQL_BP_OTHER_HSTMT) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CATALOG_TERM",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char catalogTerm[64];
  SQLSMALLINT termLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CATALOG_TERM, catalogTerm, sizeof(catalogTerm), &termLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(catalogTerm) == "database");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_COLLATION_SEQ",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char collSeq[256];
  ret = SQLGetInfo(dbc_handle(), SQL_COLLATION_SEQ, collSeq, sizeof(collSeq), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(collSeq) == "UTF-32LE_BINARY");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONCAT_NULL_BEHAVIOR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT concatBehavior = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONCAT_NULL_BEHAVIOR, &concatBehavior, sizeof(concatBehavior), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(concatBehavior == SQL_CB_NULL); // NULL concat yields NULL

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CURSOR_COMMIT_BEHAVIOR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT cursorCommit = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CURSOR_COMMIT_BEHAVIOR, &cursorCommit, sizeof(cursorCommit), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(cursorCommit == SQL_CB_CLOSE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CURSOR_ROLLBACK_BEHAVIOR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT cursorRollback = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CURSOR_ROLLBACK_BEHAVIOR, &cursorRollback, sizeof(cursorRollback), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(cursorRollback == SQL_CB_CLOSE);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CURSOR_SENSITIVITY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER sensitivity = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CURSOR_SENSITIVITY, &sensitivity, sizeof(sensitivity), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sensitivity == SQL_UNSPECIFIED);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DATA_SOURCE_READ_ONLY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char readOnly[8];
  ret = SQLGetInfo(dbc_handle(), SQL_DATA_SOURCE_READ_ONLY, readOnly, sizeof(readOnly), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(readOnly) == "N"); // Not read-only

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DEFAULT_TXN_ISOLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER txnIsolation = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DEFAULT_TXN_ISOLATION, &txnIsolation, sizeof(txnIsolation), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((txnIsolation & SQL_TXN_READ_UNCOMMITTED) == 0);
  REQUIRE((txnIsolation & SQL_TXN_READ_COMMITTED) == SQL_TXN_READ_COMMITTED); // Reference driver default is READ COMMITTED
  REQUIRE((txnIsolation & SQL_TXN_REPEATABLE_READ) == 0);
  REQUIRE((txnIsolation & SQL_TXN_SERIALIZABLE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DESCRIBE_PARAMETER",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char descParam[8];
  ret = SQLGetInfo(dbc_handle(), SQL_DESCRIBE_PARAMETER, descParam, sizeof(descParam), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(descParam) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MULT_RESULT_SETS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char multResults[8];
  ret = SQLGetInfo(dbc_handle(), SQL_MULT_RESULT_SETS, multResults, sizeof(multResults), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(multResults) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MULTIPLE_ACTIVE_TXN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char multTxn[8];
  ret = SQLGetInfo(dbc_handle(), SQL_MULTIPLE_ACTIVE_TXN, multTxn, sizeof(multTxn), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(multTxn) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_NEED_LONG_DATA_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char needLongLen[8];
  ret = SQLGetInfo(dbc_handle(), SQL_NEED_LONG_DATA_LEN, needLongLen, sizeof(needLongLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(needLongLen) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_NULL_COLLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT nullColl = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_NULL_COLLATION, &nullColl, sizeof(nullColl), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nullColl == SQL_NC_HIGH);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_PROCEDURE_TERM",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char procTerm[64];
  SQLSMALLINT termLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_PROCEDURE_TERM, procTerm, sizeof(procTerm), &termLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(termLen == 9);
  REQUIRE(std::string(procTerm) == "procedure");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SCHEMA_TERM",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char schemaTerm[64];
  SQLSMALLINT termLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SCHEMA_TERM, schemaTerm, sizeof(schemaTerm), &termLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(schemaTerm) == "schema");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SCROLL_OPTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER scrollOpts = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SCROLL_OPTIONS, &scrollOpts, sizeof(scrollOpts), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((scrollOpts & SQL_SO_FORWARD_ONLY) == SQL_SO_FORWARD_ONLY); // Reference driver supports forward-only scrolling
  REQUIRE((scrollOpts & SQL_SO_STATIC) == 0);
  REQUIRE((scrollOpts & SQL_SO_KEYSET_DRIVEN) == 0);
  REQUIRE((scrollOpts & SQL_SO_DYNAMIC) == 0);
  REQUIRE((scrollOpts & SQL_SO_MIXED) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TABLE_TERM",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char tableTerm[64];
  SQLSMALLINT termLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TABLE_TERM, tableTerm, sizeof(tableTerm), &termLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(tableTerm) == "table");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TXN_CAPABLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT txnCapable = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TXN_CAPABLE, &txnCapable, sizeof(txnCapable), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(txnCapable == SQL_TC_DDL_COMMIT); // Reference driver supports transactions with DDL commit behavior

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TXN_ISOLATION_OPTION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER txnIsoOpts = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TXN_ISOLATION_OPTION, &txnIsoOpts, sizeof(txnIsoOpts), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((txnIsoOpts & SQL_TXN_READ_UNCOMMITTED) == 0);
  REQUIRE((txnIsoOpts & SQL_TXN_READ_COMMITTED) == SQL_TXN_READ_COMMITTED); // Reference driver supports READ COMMITTED isolation
  REQUIRE((txnIsoOpts & SQL_TXN_REPEATABLE_READ) == 0);
  REQUIRE((txnIsoOpts & SQL_TXN_SERIALIZABLE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_USER_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char userName[256];
  SQLSMALLINT nameLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_USER_NAME, userName, sizeof(userName), &nameLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nameLen > 0);
  REQUIRE(!std::string(userName).empty());

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Supported SQL
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_AGGREGATE_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER aggFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_AGGREGATE_FUNCTIONS, &aggFuncs, sizeof(aggFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE((aggFuncs & SQL_AF_ALL) == SQL_AF_ALL);
  REQUIRE((aggFuncs & SQL_AF_AVG) == SQL_AF_AVG);
  REQUIRE((aggFuncs & SQL_AF_COUNT) == SQL_AF_COUNT);
  REQUIRE((aggFuncs & SQL_AF_MAX) == SQL_AF_MAX);
  REQUIRE((aggFuncs & SQL_AF_MIN) == SQL_AF_MIN);
  REQUIRE((aggFuncs & SQL_AF_SUM) == SQL_AF_SUM);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ALTER_DOMAIN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER alterDomain = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ALTER_DOMAIN, &alterDomain, sizeof(alterDomain), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support ALTER DOMAIN statement
  REQUIRE((alterDomain & SQL_AD_ADD_DOMAIN_CONSTRAINT) == 0);
  REQUIRE((alterDomain & SQL_AD_ADD_DOMAIN_DEFAULT) == 0);
  REQUIRE((alterDomain & SQL_AD_CONSTRAINT_NAME_DEFINITION) == 0);
  REQUIRE((alterDomain & SQL_AD_DROP_DOMAIN_CONSTRAINT) == 0);
  REQUIRE((alterDomain & SQL_AD_DROP_DOMAIN_DEFAULT) == 0);
  REQUIRE((alterDomain & SQL_AD_ADD_CONSTRAINT_DEFERRABLE) == 0);
  REQUIRE((alterDomain & SQL_AD_ADD_CONSTRAINT_NON_DEFERRABLE) == 0);
  REQUIRE((alterDomain & SQL_AD_ADD_CONSTRAINT_INITIALLY_DEFERRED) == 0);
  REQUIRE((alterDomain & SQL_AD_ADD_CONSTRAINT_INITIALLY_IMMEDIATE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ALTER_TABLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER alterTable = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ALTER_TABLE, &alterTable, sizeof(alterTable), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((alterTable & SQL_AT_ADD_COLUMN_COLLATION) == 0);
  REQUIRE((alterTable & SQL_AT_ADD_COLUMN_DEFAULT) == SQL_AT_ADD_COLUMN_DEFAULT); // Reference driver supports adding columns with defaults
  REQUIRE((alterTable & SQL_AT_ADD_COLUMN_SINGLE) == SQL_AT_ADD_COLUMN_SINGLE); // Reference driver supports adding single columns
  REQUIRE((alterTable & SQL_AT_ADD_CONSTRAINT) == 0);
  REQUIRE((alterTable & SQL_AT_ADD_TABLE_CONSTRAINT) == SQL_AT_ADD_TABLE_CONSTRAINT); // Reference driver supports adding table constraints
  REQUIRE((alterTable & SQL_AT_CONSTRAINT_NAME_DEFINITION) == SQL_AT_CONSTRAINT_NAME_DEFINITION); // Reference driver supports naming constraints
  REQUIRE((alterTable & SQL_AT_DROP_COLUMN_CASCADE) == SQL_AT_DROP_COLUMN_CASCADE); // Reference driver supports DROP COLUMN CASCADE
  REQUIRE((alterTable & SQL_AT_DROP_COLUMN_DEFAULT) == SQL_AT_DROP_COLUMN_DEFAULT); // Reference driver supports altering column defaults (drop)
  REQUIRE((alterTable & SQL_AT_DROP_COLUMN_RESTRICT) == SQL_AT_DROP_COLUMN_RESTRICT); // Reference driver supports DROP COLUMN RESTRICT
  REQUIRE((alterTable & SQL_AT_DROP_TABLE_CONSTRAINT_CASCADE) == SQL_AT_DROP_TABLE_CONSTRAINT_CASCADE); // Reference driver supports DROP TABLE CONSTRAINT CASCADE
  REQUIRE((alterTable & SQL_AT_DROP_TABLE_CONSTRAINT_RESTRICT) == SQL_AT_DROP_TABLE_CONSTRAINT_RESTRICT); // Reference driver supports DROP TABLE CONSTRAINT RESTRICT
  REQUIRE((alterTable & SQL_AT_SET_COLUMN_DEFAULT) == 0);
  REQUIRE((alterTable & SQL_AT_CONSTRAINT_INITIALLY_DEFERRED) == SQL_AT_CONSTRAINT_INITIALLY_DEFERRED); // Reference driver supports initially deferred constraints
  REQUIRE((alterTable & SQL_AT_CONSTRAINT_INITIALLY_IMMEDIATE) == SQL_AT_CONSTRAINT_INITIALLY_IMMEDIATE); // Reference driver supports initially immediate constraints
  REQUIRE((alterTable & SQL_AT_CONSTRAINT_DEFERRABLE) == SQL_AT_CONSTRAINT_DEFERRABLE); // Reference driver supports deferrable constraints
  REQUIRE((alterTable & SQL_AT_CONSTRAINT_NON_DEFERRABLE) == SQL_AT_CONSTRAINT_NON_DEFERRABLE); // Reference driver supports non-deferrable constraints

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DATETIME_LITERALS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER datetimeLiterals = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DATETIME_LITERALS, &datetimeLiterals, sizeof(datetimeLiterals), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support SQL-92 datetime literals
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_DATE) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_TIME) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_TIMESTAMP) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_YEAR) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_MONTH) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_DAY) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_HOUR) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_MINUTE) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_SECOND) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_YEAR_TO_MONTH) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_DAY_TO_HOUR) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_DAY_TO_MINUTE) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_DAY_TO_SECOND) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_HOUR_TO_MINUTE) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_HOUR_TO_SECOND) == 0);
  REQUIRE((datetimeLiterals & SQL_DL_SQL92_INTERVAL_MINUTE_TO_SECOND) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CATALOG_LOCATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT catLoc = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CATALOG_LOCATION, &catLoc, sizeof(catLoc), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(catLoc == SQL_CL_START); // database.schema.table

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CATALOG_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char catName[8];
  ret = SQLGetInfo(dbc_handle(), SQL_CATALOG_NAME, catName, sizeof(catName), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(catName) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CATALOG_NAME_SEPARATOR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char separator[8];
  SQLSMALLINT sepLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CATALOG_NAME_SEPARATOR, separator, sizeof(separator), &sepLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(separator) == ".");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CATALOG_USAGE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER catUsage = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CATALOG_USAGE, &catUsage, sizeof(catUsage), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((catUsage & SQL_CU_DML_STATEMENTS) == SQL_CU_DML_STATEMENTS); // Reference driver supports catalogs in DML statements
  REQUIRE((catUsage & SQL_CU_PROCEDURE_INVOCATION) == 0);
  REQUIRE((catUsage & SQL_CU_TABLE_DEFINITION) == SQL_CU_TABLE_DEFINITION); // Reference driver supports catalogs in table definition statements
  REQUIRE((catUsage & SQL_CU_INDEX_DEFINITION) == 0);
  REQUIRE((catUsage & SQL_CU_PRIVILEGE_DEFINITION) == SQL_CU_PRIVILEGE_DEFINITION); // Reference driver supports catalogs in privilege definition statements

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_COLUMN_ALIAS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char colAlias[8];
  ret = SQLGetInfo(dbc_handle(), SQL_COLUMN_ALIAS, colAlias, sizeof(colAlias), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(colAlias) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CORRELATION_NAME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT corrName = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CORRELATION_NAME, &corrName, sizeof(corrName), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(corrName == SQL_CN_ANY); // Reference driver supports any correlation name usage

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_ASSERTION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_ASSERTION, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support CREATE ASSERTION statement
  REQUIRE((create & SQL_CA_CREATE_ASSERTION) == 0);
  REQUIRE((create & SQL_CA_CONSTRAINT_INITIALLY_DEFERRED) == 0);
  REQUIRE((create & SQL_CA_CONSTRAINT_INITIALLY_IMMEDIATE) == 0);
  REQUIRE((create & SQL_CA_CONSTRAINT_DEFERRABLE) == 0);
  REQUIRE((create & SQL_CA_CONSTRAINT_NON_DEFERRABLE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_CHARACTER_SET",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_CHARACTER_SET, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support CREATE CHARACTER SET statement
  REQUIRE((create & SQL_CCS_CREATE_CHARACTER_SET) == 0);
  REQUIRE((create & SQL_CCS_COLLATE_CLAUSE) == 0);
  REQUIRE((create & SQL_CCS_LIMITED_COLLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_COLLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_COLLATION, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support CREATE COLLATION statement
  REQUIRE((create & SQL_CCOL_CREATE_COLLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_DOMAIN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_DOMAIN, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support CREATE DOMAIN statement
  REQUIRE((create & SQL_CDO_CREATE_DOMAIN) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT_NAME_DEFINITION) == 0);
  REQUIRE((create & SQL_CDO_DEFAULT) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT) == 0);
  REQUIRE((create & SQL_CDO_COLLATION) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT_INITIALLY_DEFERRED) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT_INITIALLY_IMMEDIATE) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT_DEFERRABLE) == 0);
  REQUIRE((create & SQL_CDO_CONSTRAINT_NON_DEFERRABLE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_SCHEMA",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_SCHEMA, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((create & SQL_CS_CREATE_SCHEMA) == SQL_CS_CREATE_SCHEMA); // Reference driver supports CREATE SCHEMA statement
  REQUIRE((create & SQL_CS_AUTHORIZATION) == SQL_CS_AUTHORIZATION); // Reference driver supports AUTHORIZATION clause
  REQUIRE((create & SQL_CS_DEFAULT_CHARACTER_SET) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_TABLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_TABLE, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((create & SQL_CT_CREATE_TABLE) == SQL_CT_CREATE_TABLE); // Reference driver supports CREATE TABLE statement
  REQUIRE((create & SQL_CT_TABLE_CONSTRAINT) == SQL_CT_TABLE_CONSTRAINT); // Reference driver supports specifying table constraints
  REQUIRE((create & SQL_CT_CONSTRAINT_NAME_DEFINITION) == SQL_CT_CONSTRAINT_NAME_DEFINITION); // Reference driver supports naming column and table constraints
  REQUIRE((create & SQL_CT_COMMIT_PRESERVE) == 0);
  REQUIRE((create & SQL_CT_COMMIT_DELETE) == 0);
  REQUIRE((create & SQL_CT_GLOBAL_TEMPORARY) == 0);
  REQUIRE((create & SQL_CT_LOCAL_TEMPORARY) == 0);
  REQUIRE((create & SQL_CT_COLUMN_CONSTRAINT) == SQL_CT_COLUMN_CONSTRAINT); // Reference driver supports specifying column constraints
  REQUIRE((create & SQL_CT_COLUMN_DEFAULT) == 0);
  REQUIRE((create & SQL_CT_COLUMN_COLLATION) == 0);
  REQUIRE((create & SQL_CT_CONSTRAINT_INITIALLY_DEFERRED) == SQL_CT_CONSTRAINT_INITIALLY_DEFERRED); // Reference driver supports initially deferred constraints
  REQUIRE((create & SQL_CT_CONSTRAINT_INITIALLY_IMMEDIATE) == SQL_CT_CONSTRAINT_INITIALLY_IMMEDIATE); // Reference driver supports initially immediate constraints
  REQUIRE((create & SQL_CT_CONSTRAINT_DEFERRABLE) == SQL_CT_CONSTRAINT_DEFERRABLE); // Reference driver supports deferrable constraints
  REQUIRE((create & SQL_CT_CONSTRAINT_NON_DEFERRABLE) == SQL_CT_CONSTRAINT_NON_DEFERRABLE); // Reference driver supports non-deferrable constraints

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_TRANSLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_TRANSLATION, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support CREATE TRANSLATION statement
  REQUIRE((create & SQL_CTR_CREATE_TRANSLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CREATE_VIEW",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER create = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CREATE_VIEW, &create, sizeof(create), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((create & SQL_CV_CREATE_VIEW) == SQL_CV_CREATE_VIEW); // Reference driver supports CREATE VIEW statement
  REQUIRE((create & SQL_CV_CHECK_OPTION) == 0);
  REQUIRE((create & SQL_CV_CASCADED) == 0);
  REQUIRE((create & SQL_CV_LOCAL) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DDL_INDEX",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER ddlIndex = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DDL_INDEX, &ddlIndex, sizeof(ddlIndex), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ddlIndex == 0); // Reference driver does not support DDL statements for indexes

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_ASSERTION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_ASSERTION, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support DROP ASSERTION statement
  REQUIRE((drop & SQL_DA_DROP_ASSERTION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_CHARACTER_SET",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_CHARACTER_SET, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support DROP CHARACTER SET statement
  REQUIRE((drop & SQL_DCS_DROP_CHARACTER_SET) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_COLLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_COLLATION, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support DROP COLLATION statement
  REQUIRE((drop & SQL_DC_DROP_COLLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_DOMAIN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_DOMAIN, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support DROP DOMAIN statement
  REQUIRE((drop & SQL_DD_DROP_DOMAIN) == 0);
  REQUIRE((drop & SQL_DD_CASCADE) == 0);
  REQUIRE((drop & SQL_DD_RESTRICT) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_SCHEMA",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_SCHEMA, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE((drop & SQL_DS_DROP_SCHEMA) == SQL_DS_DROP_SCHEMA); // Reference driver supports DROP SCHEMA statement
  REQUIRE((drop & SQL_DS_CASCADE) == SQL_DS_CASCADE); // Reference driver supports DROP SCHEMA CASCADE
  REQUIRE((drop & SQL_DS_RESTRICT) == SQL_DS_RESTRICT); // Reference driver supports DROP SCHEMA RESTRICT
  
  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_TABLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_TABLE, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((drop & SQL_DT_DROP_TABLE) == SQL_DT_DROP_TABLE); // Reference driver supports DROP TABLE statement
  REQUIRE((drop & SQL_DT_CASCADE) == SQL_DT_CASCADE); // Reference driver supports CASCADE option
  REQUIRE((drop & SQL_DT_RESTRICT) == SQL_DT_RESTRICT); // Reference driver supports RESTRICT option

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_TRANSLATION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_TRANSLATION, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);

  // Reference driver does not support DROP TRANSLATION statement
  REQUIRE((drop & SQL_DTR_DROP_TRANSLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_DROP_VIEW",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER drop = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DROP_VIEW, &drop, sizeof(drop), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((drop & SQL_DV_DROP_VIEW) == SQL_DV_DROP_VIEW); // Reference driver supports DROP VIEW statement
  REQUIRE((drop & SQL_DV_CASCADE) == 0);
  REQUIRE((drop & SQL_DV_RESTRICT) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_EXPRESSIONS_IN_ORDERBY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char exprOrderBy[8];
  ret = SQLGetInfo(dbc_handle(), SQL_EXPRESSIONS_IN_ORDERBY, exprOrderBy, sizeof(exprOrderBy), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(exprOrderBy) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_GROUP_BY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT groupBy = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_GROUP_BY, &groupBy, sizeof(groupBy), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(groupBy == SQL_GB_GROUP_BY_CONTAINS_SELECT); // Reference driver supports GROUP BY with expressions not in SELECT list

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_IDENTIFIER_CASE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT identCase = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_IDENTIFIER_CASE, &identCase, sizeof(identCase), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(identCase == SQL_IC_UPPER); // Stored in uppercase

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_IDENTIFIER_QUOTE_CHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char quoteChar[8];
  SQLSMALLINT quoteLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_IDENTIFIER_QUOTE_CHAR, quoteChar, sizeof(quoteChar), &quoteLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(quoteLen > 0);
  REQUIRE(std::string(quoteChar) == "\""); // SQL-92 double quote

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_INDEX_KEYWORDS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER indexKeywords = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_INDEX_KEYWORDS, &indexKeywords, sizeof(indexKeywords), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indexKeywords == SQL_IK_NONE); // Reference driver does not treat any keywords as reserved for index names

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_INSERT_STATEMENT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER insertStmt = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_INSERT_STATEMENT, &insertStmt, sizeof(insertStmt), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((insertStmt & SQL_IS_INSERT_LITERALS) == SQL_IS_INSERT_LITERALS); // Reference driver supports INSERT with literals
  REQUIRE((insertStmt & SQL_IS_INSERT_SEARCHED) == SQL_IS_INSERT_SEARCHED); // Reference driver supports INSERT with searched values
  REQUIRE((insertStmt & SQL_IS_SELECT_INTO) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_INTEGRITY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char integrity[8];
  ret = SQLGetInfo(dbc_handle(), SQL_INTEGRITY, integrity, sizeof(integrity), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(integrity) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_KEYWORDS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char keywords[4096];
  SQLSMALLINT keywordsLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_KEYWORDS, keywords, sizeof(keywords), &keywordsLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(keywordsLen == 0); // Reference driver does not have any keywords beyond the SQL-92 reserved words
  REQUIRE(std::string(keywords).empty());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_LIKE_ESCAPE_CLAUSE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char likeEscape[8];
  ret = SQLGetInfo(dbc_handle(), SQL_LIKE_ESCAPE_CLAUSE, likeEscape, sizeof(likeEscape), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(likeEscape) == "Y");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_NON_NULLABLE_COLUMNS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT nonNull = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_NON_NULLABLE_COLUMNS, &nonNull, sizeof(nonNull), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nonNull == SQL_NNC_NULL); // Reference driver does not require columns to be defined as non-nullable

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_OJ_CAPABILITIES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER ojCaps = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_OJ_CAPABILITIES, &ojCaps, sizeof(ojCaps), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((ojCaps & SQL_OJ_LEFT) == SQL_OJ_LEFT); // Reference driver supports left outer joins
  REQUIRE((ojCaps & SQL_OJ_RIGHT) == SQL_OJ_RIGHT); // Reference driver supports right outer joins
  REQUIRE((ojCaps & SQL_OJ_FULL) == SQL_OJ_FULL); // Reference driver supports full outer joins
  REQUIRE((ojCaps & SQL_OJ_NESTED) == 0);
  REQUIRE((ojCaps & SQL_OJ_NOT_ORDERED) == 0);
  REQUIRE((ojCaps & SQL_OJ_INNER) == 0);
  REQUIRE((ojCaps & SQL_OJ_ALL_COMPARISON_OPS) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ORDER_BY_COLUMNS_IN_SELECT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char orderByInSelect[8];
  ret = SQLGetInfo(dbc_handle(), SQL_ORDER_BY_COLUMNS_IN_SELECT, orderByInSelect, sizeof(orderByInSelect), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(orderByInSelect) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_PROCEDURES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char procedures[8];
  ret = SQLGetInfo(dbc_handle(), SQL_PROCEDURES, procedures, sizeof(procedures), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(procedures) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_QUOTED_IDENTIFIER_CASE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT quotedCase = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_QUOTED_IDENTIFIER_CASE, &quotedCase, sizeof(quotedCase), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(quotedCase == SQL_IC_SENSITIVE); // Reference driver treats quoted identifiers as case-sensitive

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SCHEMA_USAGE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER schemaUsage = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SCHEMA_USAGE, &schemaUsage, sizeof(schemaUsage), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((schemaUsage & SQL_SU_DML_STATEMENTS) == SQL_SU_DML_STATEMENTS); // Reference driver supports schemas in DML statements
  REQUIRE((schemaUsage & SQL_SU_PROCEDURE_INVOCATION) == 0);
  REQUIRE((schemaUsage & SQL_SU_TABLE_DEFINITION) == SQL_SU_TABLE_DEFINITION); // Reference driver supports schemas in table definition statements
  REQUIRE((schemaUsage & SQL_SU_INDEX_DEFINITION) == 0);
  REQUIRE((schemaUsage & SQL_SU_PRIVILEGE_DEFINITION) == SQL_SU_PRIVILEGE_DEFINITION); // Reference driver supports schemas in privilege definition statements

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SPECIAL_CHARACTERS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char specialChars[256];
  SQLSMALLINT specialCharsLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SPECIAL_CHARACTERS, specialChars, sizeof(specialChars), &specialCharsLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(specialCharsLen == 0); // Reference driver does not have any special characters beyond the SQL-92 reserved characters
  REQUIRE(std::string(specialChars).empty());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL_CONFORMANCE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER conformance = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL_CONFORMANCE, &conformance, sizeof(conformance), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(conformance == SQL_SC_SQL92_ENTRY); // Reference driver conforms to SQL-92 Entry level

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_STANDARD_CLI_CONFORMANCE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER cliConf = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_STANDARD_CLI_CONFORMANCE, &cliConf, sizeof(cliConf), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(cliConf == SQL_SCC_ISO92_CLI); // Reference driver conforms to ISO 92 CLI standard

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SUBQUERIES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER subqueries = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SUBQUERIES, &subqueries, sizeof(subqueries), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((subqueries & SQL_SQ_CORRELATED_SUBQUERIES) == SQL_SQ_CORRELATED_SUBQUERIES); // Reference driver supports correlated subqueries
  REQUIRE((subqueries & SQL_SQ_COMPARISON) == SQL_SQ_COMPARISON); // Reference driver supports subqueries in comparison predicates
  REQUIRE((subqueries & SQL_SQ_EXISTS) == SQL_SQ_EXISTS); // Reference driver supports subqueries in EXISTS predicates
  REQUIRE((subqueries & SQL_SQ_IN) == SQL_SQ_IN); // Reference driver supports subqueries in IN predicates
  REQUIRE((subqueries & SQL_SQ_QUANTIFIED) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_UNION",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER unionSupport = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_UNION, &unionSupport, sizeof(unionSupport), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((unionSupport & SQL_U_UNION) == SQL_U_UNION); // Reference driver supports UNION clause
  REQUIRE((unionSupport & SQL_U_UNION_ALL) == SQL_U_UNION_ALL); // Reference driver supports UNION ALL clause

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_FOREIGN_KEY_DELETE_RULE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER rules = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_FOREIGN_KEY_DELETE_RULE, &rules, sizeof(rules), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((rules & SQL_SFKD_CASCADE) == SQL_SFKD_CASCADE); // Reference driver supports CASCADE on DELETE
  REQUIRE((rules & SQL_SFKD_NO_ACTION) == SQL_SFKD_NO_ACTION); // Reference driver supports NO ACTION on DELETE
  REQUIRE((rules & SQL_SFKD_SET_DEFAULT) == SQL_SFKD_SET_DEFAULT); // Reference driver supports SET DEFAULT on DELETE
  REQUIRE((rules & SQL_SFKD_SET_NULL) == SQL_SFKD_SET_NULL); // Reference driver supports SET NULL on DELETE

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_FOREIGN_KEY_UPDATE_RULE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER rules = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_FOREIGN_KEY_UPDATE_RULE, &rules, sizeof(rules), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((rules & SQL_SFKU_CASCADE) == SQL_SFKU_CASCADE); // Reference driver supports CASCADE on UPDATE
  REQUIRE((rules & SQL_SFKU_NO_ACTION) == SQL_SFKU_NO_ACTION); // Reference driver supports NO ACTION on UPDATE
  REQUIRE((rules & SQL_SFKU_SET_DEFAULT) == SQL_SFKU_SET_DEFAULT); // Reference driver supports SET DEFAULT on UPDATE
  REQUIRE((rules & SQL_SFKU_SET_NULL) == SQL_SFKU_SET_NULL); // Reference driver supports SET NULL on UPDATE

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_GRANT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER grantSupport = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_GRANT, &grantSupport, sizeof(grantSupport), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((grantSupport & SQL_SG_DELETE_TABLE) == SQL_SG_DELETE_TABLE); // Reference driver supports GRANT DELETE on tables
  REQUIRE((grantSupport & SQL_SG_INSERT_COLUMN) == 0);
  REQUIRE((grantSupport & SQL_SG_INSERT_TABLE) == SQL_SG_INSERT_TABLE); // Reference driver supports GRANT INSERT on tables
  REQUIRE((grantSupport & SQL_SG_REFERENCES_TABLE) == SQL_SG_REFERENCES_TABLE); // Reference driver supports GRANT REFERENCES on tables
  REQUIRE((grantSupport & SQL_SG_REFERENCES_COLUMN) == 0);
  REQUIRE((grantSupport & SQL_SG_SELECT_TABLE) == SQL_SG_SELECT_TABLE); // Reference driver supports GRANT SELECT on tables
  REQUIRE((grantSupport & SQL_SG_UPDATE_COLUMN) == 0);
  REQUIRE((grantSupport & SQL_SG_UPDATE_TABLE) == SQL_SG_UPDATE_TABLE); // Reference driver supports GRANT UPDATE on tables
  REQUIRE((grantSupport & SQL_SG_USAGE_ON_DOMAIN) == 0);
  REQUIRE((grantSupport & SQL_SG_USAGE_ON_CHARACTER_SET) == 0);
  REQUIRE((grantSupport & SQL_SG_USAGE_ON_COLLATION) == 0);
  REQUIRE((grantSupport & SQL_SG_USAGE_ON_TRANSLATION) == 0);
  REQUIRE((grantSupport & SQL_SG_WITH_GRANT_OPTION) == SQL_SG_WITH_GRANT_OPTION); // Reference driver supports GRANT WITH GRANT OPTION

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_PREDICATES",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER preds = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_PREDICATES, &preds, sizeof(preds), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((preds & SQL_SP_BETWEEN) == SQL_SP_BETWEEN); // Reference driver supports BETWEEN predicate
  REQUIRE((preds & SQL_SP_COMPARISON) == SQL_SP_COMPARISON); // Reference driver supports comparison predicates
  REQUIRE((preds & SQL_SP_EXISTS) == SQL_SP_EXISTS); // Reference driver supports EXISTS predicate
  REQUIRE((preds & SQL_SP_IN) == SQL_SP_IN); // Reference driver supports IN predicate
  REQUIRE((preds & SQL_SP_ISNOTNULL) == 0);
  REQUIRE((preds & SQL_SP_ISNULL) == SQL_SP_ISNULL); // Reference driver supports IS NULL predicate
  REQUIRE((preds & SQL_SP_LIKE) == SQL_SP_LIKE); // Reference driver supports LIKE predicate
  REQUIRE((preds & SQL_SP_MATCH_FULL) == 0);
  REQUIRE((preds & SQL_SP_MATCH_PARTIAL) == 0);
  REQUIRE((preds & SQL_SP_MATCH_UNIQUE_FULL) == 0);
  REQUIRE((preds & SQL_SP_MATCH_UNIQUE_PARTIAL) == 0);
  REQUIRE((preds & SQL_SP_OVERLAPS) == 0);
  REQUIRE((preds & SQL_SP_QUANTIFIED_COMPARISON) == SQL_SP_QUANTIFIED_COMPARISON); // Reference driver supports quantified comparison predicates
  REQUIRE((preds & SQL_SP_UNIQUE) == SQL_SP_UNIQUE); // Reference driver supports UNIQUE predicate

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_RELATIONAL_JOIN_OPERATORS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER joinOps = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_RELATIONAL_JOIN_OPERATORS, &joinOps, sizeof(joinOps), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((joinOps & SQL_SRJO_CORRESPONDING_CLAUSE) == 0);
  REQUIRE((joinOps & SQL_SRJO_CROSS_JOIN) == SQL_SRJO_CROSS_JOIN); // Reference driver supports CROSS JOIN
  REQUIRE((joinOps & SQL_SRJO_EXCEPT_JOIN) == 0);
  REQUIRE((joinOps & SQL_SRJO_FULL_OUTER_JOIN) == SQL_SRJO_FULL_OUTER_JOIN); // Reference driver supports FULL OUTER JOIN
  REQUIRE((joinOps & SQL_SRJO_INNER_JOIN) == SQL_SRJO_INNER_JOIN); // Reference driver supports INNER JOIN
  REQUIRE((joinOps & SQL_SRJO_INTERSECT_JOIN) == 0);
  REQUIRE((joinOps & SQL_SRJO_LEFT_OUTER_JOIN) == SQL_SRJO_LEFT_OUTER_JOIN); // Reference driver supports LEFT OUTER JOIN
  REQUIRE((joinOps & SQL_SRJO_NATURAL_JOIN) == 0);
  REQUIRE((joinOps & SQL_SRJO_RIGHT_OUTER_JOIN) == SQL_SRJO_RIGHT_OUTER_JOIN); // Reference driver supports RIGHT OUTER JOIN
  REQUIRE((joinOps & SQL_SRJO_UNION_JOIN) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_REVOKE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER revokeSupport = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_REVOKE, &revokeSupport, sizeof(revokeSupport), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((revokeSupport & SQL_SR_CASCADE) == SQL_SR_CASCADE); // Reference driver supports REVOKE with CASCADE
  REQUIRE((revokeSupport & SQL_SR_DELETE_TABLE) == SQL_SR_DELETE_TABLE); // Reference driver supports REVOKE DELETE on tables
  REQUIRE((revokeSupport & SQL_SR_GRANT_OPTION_FOR) == SQL_SR_GRANT_OPTION_FOR); // Reference driver supports REVOKE GRANT OPTION FOR
  REQUIRE((revokeSupport & SQL_SR_INSERT_COLUMN) == 0);
  REQUIRE((revokeSupport & SQL_SR_INSERT_TABLE) == SQL_SR_INSERT_TABLE); // Reference driver supports REVOKE INSERT on tables
  REQUIRE((revokeSupport & SQL_SR_REFERENCES_COLUMN) == 0);
  REQUIRE((revokeSupport & SQL_SR_REFERENCES_TABLE) == SQL_SR_REFERENCES_TABLE); // Reference driver supports REVOKE REFERENCES on tables
  REQUIRE((revokeSupport & SQL_SR_RESTRICT) == SQL_SR_RESTRICT); // Reference driver supports REVOKE with RESTRICT
  REQUIRE((revokeSupport & SQL_SR_SELECT_TABLE) == SQL_SR_SELECT_TABLE); // Reference driver supports REVOKE SELECT on tables
  REQUIRE((revokeSupport & SQL_SR_UPDATE_COLUMN) == 0);
  REQUIRE((revokeSupport & SQL_SR_UPDATE_TABLE) == SQL_SR_UPDATE_TABLE); // Reference driver supports REVOKE UPDATE on tables
  REQUIRE((revokeSupport & SQL_SR_USAGE_ON_DOMAIN) == 0);
  REQUIRE((revokeSupport & SQL_SR_USAGE_ON_CHARACTER_SET) == 0);
  REQUIRE((revokeSupport & SQL_SR_USAGE_ON_COLLATION) == 0);
  REQUIRE((revokeSupport & SQL_SR_USAGE_ON_TRANSLATION) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_ROW_VALUE_CONSTRUCTOR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER rowValConstr = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_ROW_VALUE_CONSTRUCTOR, &rowValConstr, sizeof(rowValConstr), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((rowValConstr & SQL_SRVC_VALUE_EXPRESSION) == SQL_SRVC_VALUE_EXPRESSION); // Reference driver supports value expressions in row value constructors
  REQUIRE((rowValConstr & SQL_SRVC_NULL) == SQL_SRVC_NULL); // Reference driver supports NULL in row value constructors
  REQUIRE((rowValConstr & SQL_SRVC_DEFAULT) == SQL_SRVC_DEFAULT); // Reference driver supports DEFAULT in row value constructors
  REQUIRE((rowValConstr & SQL_SRVC_ROW_SUBQUERY) == SQL_SRVC_ROW_SUBQUERY); // Reference driver supports row subqueries in row value constructors

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_VALUE_EXPRESSIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER valExprs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_VALUE_EXPRESSIONS, &valExprs, sizeof(valExprs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((valExprs & SQL_SVE_CASE) == SQL_SVE_CASE); // Reference driver supports CASE expressions
  REQUIRE((valExprs & SQL_SVE_CAST) == SQL_SVE_CAST); // Reference driver supports CAST expressions
  REQUIRE((valExprs & SQL_SVE_COALESCE) == SQL_SVE_COALESCE); // Reference driver supports COALESCE expressions
  REQUIRE((valExprs & SQL_SVE_NULLIF) == SQL_SVE_NULLIF); // Reference driver supports NULLIF expressions

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_XOPEN_CLI_YEAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char year[16];
  ret = SQLGetInfo(dbc_handle(), SQL_XOPEN_CLI_YEAR, year, sizeof(year), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(year) == "1995"); // Reference driver supports XOPEN CLI year 1995

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQL Limits
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_BINARY_LITERAL_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxBinaryLit = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_BINARY_LITERAL_LEN, &maxBinaryLit, sizeof(maxBinaryLit), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxBinaryLit == 0); // Reference driver does not support binary literals

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_CATALOG_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxCatLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_CATALOG_NAME_LEN, &maxCatLen, sizeof(maxCatLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxCatLen == 255);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_CHAR_LITERAL_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxCharLit = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_CHAR_LITERAL_LEN, &maxCharLit, sizeof(maxCharLit), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxCharLit == 16777216);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMN_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMN_NAME_LEN, &maxColLen, sizeof(maxColLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColLen == 255);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMNS_IN_GROUP_BY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColsGroupBy = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMNS_IN_GROUP_BY, &maxColsGroupBy, sizeof(maxColsGroupBy), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColsGroupBy == 65535);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMNS_IN_INDEX",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColsIndex = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMNS_IN_INDEX, &maxColsIndex, sizeof(maxColsIndex), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColsIndex == 0); // No specified limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMNS_IN_ORDER_BY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColsOrderBy = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMNS_IN_ORDER_BY, &maxColsOrderBy, sizeof(maxColsOrderBy), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColsOrderBy == 65535);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMNS_IN_SELECT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColsSelect = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMNS_IN_SELECT, &maxColsSelect, sizeof(maxColsSelect), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColsSelect == 65535);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_COLUMNS_IN_TABLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxColsTable = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_COLUMNS_IN_TABLE, &maxColsTable, sizeof(maxColsTable), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxColsTable == 65535);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_CURSOR_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxCursorLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_CURSOR_NAME_LEN, &maxCursorLen, sizeof(maxCursorLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxCursorLen == 0); // No specified limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_IDENTIFIER_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxIdent = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_IDENTIFIER_LEN, &maxIdent, sizeof(maxIdent), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxIdent == 255);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_INDEX_SIZE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxIndexSize = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_INDEX_SIZE, &maxIndexSize, sizeof(maxIndexSize), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxIndexSize == 0); // No specified limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_PROCEDURE_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxProcLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_PROCEDURE_NAME_LEN, &maxProcLen, sizeof(maxProcLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxProcLen == 0); // No specified limit or unknown, reference driver does not support stored procedures

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_ROW_SIZE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxRowSize = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_ROW_SIZE, &maxRowSize, sizeof(maxRowSize), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxRowSize == 16777216);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_ROW_SIZE_INCLUDES_LONG",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char includesLong[8];
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_ROW_SIZE_INCLUDES_LONG, includesLong, sizeof(includesLong), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(includesLong) == "N");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_SCHEMA_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxSchemaLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_SCHEMA_NAME_LEN, &maxSchemaLen, sizeof(maxSchemaLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxSchemaLen == 255);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_STATEMENT_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER maxStmtLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_STATEMENT_LEN, &maxStmtLen, sizeof(maxStmtLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxStmtLen == 0); // No limit or unknown

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_TABLE_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxTableLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_TABLE_NAME_LEN, &maxTableLen, sizeof(maxTableLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxTableLen == 255);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_TABLES_IN_SELECT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxTables = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_TABLES_IN_SELECT, &maxTables, sizeof(maxTables), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxTables == 0); // No specified limit or unknown, reference driver does not have a specific limit on number of tables in a SELECT statement

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_MAX_USER_NAME_LEN",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT maxUserLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_MAX_USER_NAME_LEN, &maxUserLen, sizeof(maxUserLen), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(maxUserLen == 0); // No specified limit or unknown, reference driver does not have a specific limit on user name length

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Scalar Function Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_NUMERIC_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER numFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_NUMERIC_FUNCTIONS, &numFuncs, sizeof(numFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((numFuncs & SQL_FN_NUM_ABS) == SQL_FN_NUM_ABS); // Reference driver supports ABS function
  REQUIRE((numFuncs & SQL_FN_NUM_ACOS) == SQL_FN_NUM_ACOS); // Reference driver supports ACOS function
  REQUIRE((numFuncs & SQL_FN_NUM_ASIN) == SQL_FN_NUM_ASIN); // Reference driver supports ASIN function
  REQUIRE((numFuncs & SQL_FN_NUM_ATAN) == SQL_FN_NUM_ATAN); // Reference driver supports ATAN function
  REQUIRE((numFuncs & SQL_FN_NUM_ATAN2) == SQL_FN_NUM_ATAN2); // Reference driver supports ATAN2 function
  REQUIRE((numFuncs & SQL_FN_NUM_CEILING) == SQL_FN_NUM_CEILING); // Reference driver supports CEILING function
  REQUIRE((numFuncs & SQL_FN_NUM_COS) == SQL_FN_NUM_COS); // Reference driver supports COS function
  REQUIRE((numFuncs & SQL_FN_NUM_COT) == SQL_FN_NUM_COT); // Reference driver supports COT function
  REQUIRE((numFuncs & SQL_FN_NUM_DEGREES) == SQL_FN_NUM_DEGREES); // Reference driver supports DEGREES function
  REQUIRE((numFuncs & SQL_FN_NUM_EXP) == SQL_FN_NUM_EXP); // Reference driver supports EXP function
  REQUIRE((numFuncs & SQL_FN_NUM_FLOOR) == SQL_FN_NUM_FLOOR); // Reference driver supports FLOOR function
  REQUIRE((numFuncs & SQL_FN_NUM_LOG) == SQL_FN_NUM_LOG); // Reference driver supports LOG function
  REQUIRE((numFuncs & SQL_FN_NUM_LOG10) == 0);
  REQUIRE((numFuncs & SQL_FN_NUM_MOD) == SQL_FN_NUM_MOD); // Reference driver supports MOD function
  REQUIRE((numFuncs & SQL_FN_NUM_PI) == SQL_FN_NUM_PI); // Reference driver supports PI function
  REQUIRE((numFuncs & SQL_FN_NUM_POWER) == SQL_FN_NUM_POWER); // Reference driver supports POWER function
  REQUIRE((numFuncs & SQL_FN_NUM_RADIANS) == SQL_FN_NUM_RADIANS); // Reference driver supports RADIANS function
  REQUIRE((numFuncs & SQL_FN_NUM_RAND) == SQL_FN_NUM_RAND); // Reference driver supports RAND function
  REQUIRE((numFuncs & SQL_FN_NUM_ROUND) == SQL_FN_NUM_ROUND); // Reference driver supports ROUND function
  REQUIRE((numFuncs & SQL_FN_NUM_SIGN) == SQL_FN_NUM_SIGN); // Reference driver supports SIGN function
  REQUIRE((numFuncs & SQL_FN_NUM_SIN) == SQL_FN_NUM_SIN); // Reference driver supports SIN function
  REQUIRE((numFuncs & SQL_FN_NUM_SQRT) == SQL_FN_NUM_SQRT); // Reference driver supports SQRT function
  REQUIRE((numFuncs & SQL_FN_NUM_TAN) == SQL_FN_NUM_TAN); // Reference driver supports TAN function
  REQUIRE((numFuncs & SQL_FN_NUM_TRUNCATE) == SQL_FN_NUM_TRUNCATE); // Reference driver supports TRUNCATE function

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_STRING_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER strFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_STRING_FUNCTIONS, &strFuncs, sizeof(strFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((strFuncs & SQL_FN_STR_ASCII) == SQL_FN_STR_ASCII); // Reference driver supports ASCII function
  REQUIRE((strFuncs & SQL_FN_STR_BIT_LENGTH) == SQL_FN_STR_BIT_LENGTH); // Reference driver supports BIT_LENGTH function
  REQUIRE((strFuncs & SQL_FN_STR_CHAR) == SQL_FN_STR_CHAR); // Reference driver supports CHAR function
  REQUIRE((strFuncs & SQL_FN_STR_CHAR_LENGTH) == SQL_FN_STR_CHAR_LENGTH); // Reference driver supports CHAR_LENGTH function
  REQUIRE((strFuncs & SQL_FN_STR_CHARACTER_LENGTH) == SQL_FN_STR_CHARACTER_LENGTH); // Reference driver supports CHARACTER_LENGTH function
  REQUIRE((strFuncs & SQL_FN_STR_CONCAT) == SQL_FN_STR_CONCAT); // Reference driver supports CONCAT function
  REQUIRE((strFuncs & SQL_FN_STR_DIFFERENCE) == 0);
  REQUIRE((strFuncs & SQL_FN_STR_INSERT) == SQL_FN_STR_INSERT); // Reference driver supports INSERT function
  REQUIRE((strFuncs & SQL_FN_STR_LCASE) == SQL_FN_STR_LCASE); // Reference driver supports LCASE function
  REQUIRE((strFuncs & SQL_FN_STR_LEFT) == SQL_FN_STR_LEFT); // Reference driver supports LEFT function
  REQUIRE((strFuncs & SQL_FN_STR_LENGTH) == SQL_FN_STR_LENGTH); // Reference driver supports LENGTH function
  REQUIRE((strFuncs & SQL_FN_STR_LOCATE) == SQL_FN_STR_LOCATE); // Reference driver supports LOCATE function
  REQUIRE((strFuncs & SQL_FN_STR_LTRIM) == SQL_FN_STR_LTRIM); // Reference driver supports LTRIM function
  REQUIRE((strFuncs & SQL_FN_STR_OCTET_LENGTH) == SQL_FN_STR_OCTET_LENGTH); // Reference driver supports OCTET_LENGTH function
  REQUIRE((strFuncs & SQL_FN_STR_POSITION) == SQL_FN_STR_POSITION); // Reference driver supports POSITION function
  REQUIRE((strFuncs & SQL_FN_STR_REPEAT) == SQL_FN_STR_REPEAT); // Reference driver supports REPEAT function
  REQUIRE((strFuncs & SQL_FN_STR_REPLACE) == SQL_FN_STR_REPLACE); // Reference driver supports REPLACE function
  REQUIRE((strFuncs & SQL_FN_STR_RIGHT) == SQL_FN_STR_RIGHT); // Reference driver supports RIGHT function
  REQUIRE((strFuncs & SQL_FN_STR_RTRIM) == SQL_FN_STR_RTRIM); // Reference driver supports RTRIM function
  REQUIRE((strFuncs & SQL_FN_STR_SOUNDEX) == 0);
  REQUIRE((strFuncs & SQL_FN_STR_SPACE) == SQL_FN_STR_SPACE); // Reference driver supports SPACE function
  REQUIRE((strFuncs & SQL_FN_STR_SUBSTRING) == SQL_FN_STR_SUBSTRING); // Reference driver supports SUBSTRING function
  REQUIRE((strFuncs & SQL_FN_STR_UCASE) == SQL_FN_STR_UCASE); // Reference driver supports UCASE function

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SYSTEM_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER sysFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SYSTEM_FUNCTIONS, &sysFuncs, sizeof(sysFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((sysFuncs & SQL_FN_SYS_DBNAME) == SQL_FN_SYS_DBNAME); // Reference driver supports DBNAME function
  REQUIRE((sysFuncs & SQL_FN_SYS_IFNULL) == SQL_FN_SYS_IFNULL); // Reference driver supports IFNULL function
  REQUIRE((sysFuncs & SQL_FN_SYS_USERNAME) == SQL_FN_SYS_USERNAME); // Reference driver supports USERNAME function

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TIMEDATE_ADD_INTERVALS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER intervals = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TIMEDATE_ADD_INTERVALS, &intervals, sizeof(intervals), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((intervals & SQL_FN_TSI_FRAC_SECOND) == SQL_FN_TSI_FRAC_SECOND); // Reference driver supports fractional seconds interval
  REQUIRE((intervals & SQL_FN_TSI_SECOND) == SQL_FN_TSI_SECOND); // Reference driver supports seconds interval
  REQUIRE((intervals & SQL_FN_TSI_MINUTE) == SQL_FN_TSI_MINUTE); // Reference driver supports minutes interval
  REQUIRE((intervals & SQL_FN_TSI_HOUR) == SQL_FN_TSI_HOUR); // Reference driver supports hours interval
  REQUIRE((intervals & SQL_FN_TSI_DAY) == SQL_FN_TSI_DAY); // Reference driver supports days interval
  REQUIRE((intervals & SQL_FN_TSI_WEEK) == SQL_FN_TSI_WEEK); // Reference driver supports weeks interval
  REQUIRE((intervals & SQL_FN_TSI_MONTH) == SQL_FN_TSI_MONTH); // Reference driver supports months interval
  REQUIRE((intervals & SQL_FN_TSI_QUARTER) == SQL_FN_TSI_QUARTER); // Reference driver supports quarters interval
  REQUIRE((intervals & SQL_FN_TSI_YEAR) == SQL_FN_TSI_YEAR); // Reference driver supports years interval

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TIMEDATE_DIFF_INTERVALS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER intervals = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TIMEDATE_DIFF_INTERVALS, &intervals, sizeof(intervals), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((intervals & SQL_FN_TSI_FRAC_SECOND) == SQL_FN_TSI_FRAC_SECOND); // Reference driver supports fractional seconds interval
  REQUIRE((intervals & SQL_FN_TSI_SECOND) == SQL_FN_TSI_SECOND); // Reference driver supports seconds interval
  REQUIRE((intervals & SQL_FN_TSI_MINUTE) == SQL_FN_TSI_MINUTE); // Reference driver supports minutes interval
  REQUIRE((intervals & SQL_FN_TSI_HOUR) == SQL_FN_TSI_HOUR); // Reference driver supports hours interval
  REQUIRE((intervals & SQL_FN_TSI_DAY) == SQL_FN_TSI_DAY); // Reference driver supports days interval
  REQUIRE((intervals & SQL_FN_TSI_WEEK) == SQL_FN_TSI_WEEK); // Reference driver supports weeks interval
  REQUIRE((intervals & SQL_FN_TSI_MONTH) == SQL_FN_TSI_MONTH); // Reference driver supports months interval
  REQUIRE((intervals & SQL_FN_TSI_QUARTER) == SQL_FN_TSI_QUARTER); // Reference driver supports quarters interval
  REQUIRE((intervals & SQL_FN_TSI_YEAR) == SQL_FN_TSI_YEAR); // Reference driver supports years interval

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_TIMEDATE_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER timedateFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_TIMEDATE_FUNCTIONS, &timedateFuncs, sizeof(timedateFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((timedateFuncs & SQL_FN_TD_CURRENT_DATE) == SQL_FN_TD_CURRENT_DATE); // Reference driver supports CURRENT_DATE function
  REQUIRE((timedateFuncs & SQL_FN_TD_CURRENT_TIME) == SQL_FN_TD_CURRENT_TIME); // Reference driver supports CURRENT_TIME function
  REQUIRE((timedateFuncs & SQL_FN_TD_CURRENT_TIMESTAMP) == SQL_FN_TD_CURRENT_TIMESTAMP); // Reference driver supports CURRENT_TIMESTAMP function
  REQUIRE((timedateFuncs & SQL_FN_TD_CURDATE) == SQL_FN_TD_CURDATE); // Reference driver supports CURDATE function
  REQUIRE((timedateFuncs & SQL_FN_TD_CURTIME) == SQL_FN_TD_CURTIME); // Reference driver supports CURTIME function
  REQUIRE((timedateFuncs & SQL_FN_TD_DAYNAME) == SQL_FN_TD_DAYNAME); // Reference driver supports DAYNAME function
  REQUIRE((timedateFuncs & SQL_FN_TD_DAYOFMONTH) == SQL_FN_TD_DAYOFMONTH); // Reference driver supports DAYOFMONTH function
  REQUIRE((timedateFuncs & SQL_FN_TD_DAYOFWEEK) == SQL_FN_TD_DAYOFWEEK); // Reference driver supports DAYOFWEEK function
  REQUIRE((timedateFuncs & SQL_FN_TD_DAYOFYEAR) == SQL_FN_TD_DAYOFYEAR); // Reference driver supports DAYOFYEAR function
  REQUIRE((timedateFuncs & SQL_FN_TD_EXTRACT) == SQL_FN_TD_EXTRACT); // Reference driver supports EXTRACT function
  REQUIRE((timedateFuncs & SQL_FN_TD_HOUR) == SQL_FN_TD_HOUR); // Reference driver supports HOUR function
  REQUIRE((timedateFuncs & SQL_FN_TD_MINUTE) == SQL_FN_TD_MINUTE); // Reference driver supports MINUTE function
  REQUIRE((timedateFuncs & SQL_FN_TD_MONTH) == SQL_FN_TD_MONTH); // Reference driver supports MONTH function
  REQUIRE((timedateFuncs & SQL_FN_TD_MONTHNAME) == SQL_FN_TD_MONTHNAME); // Reference driver supports MONTHNAME function
  REQUIRE((timedateFuncs & SQL_FN_TD_NOW) == SQL_FN_TD_NOW); // Reference driver supports NOW function
  REQUIRE((timedateFuncs & SQL_FN_TD_QUARTER) == SQL_FN_TD_QUARTER); // Reference driver supports QUARTER function
  REQUIRE((timedateFuncs & SQL_FN_TD_SECOND) == SQL_FN_TD_SECOND); // Reference driver supports SECOND function
  REQUIRE((timedateFuncs & SQL_FN_TD_TIMESTAMPADD) == SQL_FN_TD_TIMESTAMPADD); // Reference driver supports TIMESTAMPADD function
  REQUIRE((timedateFuncs & SQL_FN_TD_TIMESTAMPDIFF) == SQL_FN_TD_TIMESTAMPDIFF); // Reference driver supports TIMESTAMPDIFF function
  REQUIRE((timedateFuncs & SQL_FN_TD_WEEK) == SQL_FN_TD_WEEK); // Reference driver supports WEEK function
  REQUIRE((timedateFuncs & SQL_FN_TD_YEAR) == SQL_FN_TD_YEAR); // Reference driver supports YEAR function

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_DATETIME_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER funcs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_DATETIME_FUNCTIONS, &funcs, sizeof(funcs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((funcs & SQL_SDF_CURRENT_DATE) == SQL_SDF_CURRENT_DATE); // Reference driver supports CURRENT_DATE
  REQUIRE((funcs & SQL_SDF_CURRENT_TIME) == SQL_SDF_CURRENT_TIME); // Reference driver supports CURRENT_TIME
  REQUIRE((funcs & SQL_SDF_CURRENT_TIMESTAMP) == SQL_SDF_CURRENT_TIMESTAMP); // Reference driver supports CURRENT_TIMESTAMP

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_NUMERIC_VALUE_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER funcs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_NUMERIC_VALUE_FUNCTIONS, &funcs, sizeof(funcs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((funcs & SQL_SNVF_BIT_LENGTH) == SQL_SNVF_BIT_LENGTH); // Reference driver supports BIT_LENGTH
  REQUIRE((funcs & SQL_SNVF_CHAR_LENGTH) == 0);
  REQUIRE((funcs & SQL_SNVF_CHARACTER_LENGTH) == 0);
  REQUIRE((funcs & SQL_SNVF_EXTRACT) == SQL_SNVF_EXTRACT); // Reference driver supports EXTRACT
  REQUIRE((funcs & SQL_SNVF_OCTET_LENGTH) == SQL_SNVF_OCTET_LENGTH); // Reference driver supports OCTET_LENGTH
  REQUIRE((funcs & SQL_SNVF_POSITION) == SQL_SNVF_POSITION); // Reference driver supports POSITION

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SQL92_STRING_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER funcs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SQL92_STRING_FUNCTIONS, &funcs, sizeof(funcs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((funcs & SQL_SSF_CONVERT) == SQL_SSF_CONVERT); // Reference driver supports CONVERT for strings
  REQUIRE((funcs & SQL_SSF_LOWER) == SQL_SSF_LOWER); // Reference driver supports LOWER
  REQUIRE((funcs & SQL_SSF_UPPER) == SQL_SSF_UPPER); // Reference driver supports UPPER
  REQUIRE((funcs & SQL_SSF_SUBSTRING) == SQL_SSF_SUBSTRING); // Reference driver supports SUBSTRING
  REQUIRE((funcs & SQL_SSF_TRANSLATE) == 0);
  REQUIRE((funcs & SQL_SSF_TRIM_BOTH) == SQL_SSF_TRIM_BOTH); // Reference driver supports TRIM BOTH
  REQUIRE((funcs & SQL_SSF_TRIM_LEADING) == SQL_SSF_TRIM_LEADING); // Reference driver supports TRIM LEADING
  REQUIRE((funcs & SQL_SSF_TRIM_TRAILING) == SQL_SSF_TRIM_TRAILING); // Reference driver supports TRIM TRAILING

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Conversion Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_FUNCTIONS",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convertFuncs = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_FUNCTIONS, &convertFuncs, sizeof(convertFuncs), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convertFuncs & SQL_FN_CVT_CAST) == SQL_FN_CVT_CAST); // Reference driver supports CAST function
  REQUIRE((convertFuncs & SQL_FN_CVT_CONVERT) == SQL_FN_CVT_CONVERT); // Reference driver supports CONVERT function

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_BIGINT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_BIGINT, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports BIGINT to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports BIGINT to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports BIGINT to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports BIGINT to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports BIGINT to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports BIGINT to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports BIGINT to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports BIGINT to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports BIGINT to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports BIGINT to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports BIGINT to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports BIGINT to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports BIGINT to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports BIGINT to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports BIGINT to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports BIGINT to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports BIGINT to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_BINARY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_BINARY, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports BINARY to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports BINARY to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports BINARY to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports BINARY to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_BIT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_BIT, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports BIT to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports BIT to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports BIT to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports BIT to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports BIT to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports BIT to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports BIT to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports BIT to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == SQL_CVT_LONGVARBINARY); // Reference driver supports BIT to LONGVARBINARY conversion
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports BIT to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports BIT to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports BIT to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports BIT to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports BIT to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports BIT to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports BIT to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_CHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_CHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports CHAR to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports CHAR to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports CHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports CHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports CHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports CHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports CHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports CHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports CHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports CHAR to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports CHAR to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports CHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports CHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports CHAR to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports CHAR to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports CHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_DATE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_DATE, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports DATE to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports DATE to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports DATE to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports DATE to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_DECIMAL",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_DECIMAL, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports DECIMAL to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports DECIMAL to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports DECIMAL to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports DECIMAL to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports DECIMAL to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports DECIMAL to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports DECIMAL to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_DOUBLE",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_DOUBLE, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports DOUBLE to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports DOUBLE to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports DOUBLE to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports DOUBLE to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports DOUBLE to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports DOUBLE to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports DOUBLE to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_FLOAT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_FLOAT, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports FLOAT to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports FLOAT to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports FLOAT to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports FLOAT to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports FLOAT to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports FLOAT to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports FLOAT to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_GUID",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_GUID, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports GUID to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports GUID to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == SQL_CVT_GUID); // Reference driver supports GUID to GUID conversion
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports GUID to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports GUID to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports GUID to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_INTEGER",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_INTEGER, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports INTEGER to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports INTEGER to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports INTEGER to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports INTEGER to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports INTEGER to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports INTEGER to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports INTEGER to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_INTERVAL_DAY_TIME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_INTERVAL_DAY_TIME, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports INTERVAL_DAY_TIME to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports INTERVAL_DAY_TIME to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports INTERVAL_DAY_TIME to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports INTERVAL_DAY_TIME to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports INTERVAL_DAY_TIME to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports INTERVAL_DAY_TIME to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports INTERVAL_DAY_TIME to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports INTERVAL_DAY_TIME to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports INTERVAL_DAY_TIME to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports INTERVAL_DAY_TIME to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports INTERVAL_DAY_TIME to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports INTERVAL_DAY_TIME to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_INTERVAL_YEAR_MONTH",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_INTERVAL_YEAR_MONTH, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports INTERVAL_YEAR_MONTH to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports INTERVAL_YEAR_MONTH to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports INTERVAL_YEAR_MONTH to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports INTERVAL_YEAR_MONTH to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports INTERVAL_YEAR_MONTH to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports INTERVAL_YEAR_MONTH to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports INTERVAL_YEAR_MONTH to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports INTERVAL_YEAR_MONTH to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports INTERVAL_YEAR_MONTH to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports INTERVAL_YEAR_MONTH to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports INTERVAL_YEAR_MONTH to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports INTERVAL_YEAR_MONTH to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_LONGVARBINARY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_LONGVARBINARY, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports LONGVARBINARY to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports LONGVARBINARY to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == SQL_CVT_LONGVARBINARY); // Reference driver supports LONGVARBINARY to LONGVARBINARY conversion
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports LONGVARBINARY to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports LONGVARBINARY to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports LONGVARBINARY to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_LONGVARCHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_LONGVARCHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports LONGVARCHAR to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports LONGVARCHAR to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports LONGVARCHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports LONGVARCHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports LONGVARCHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports LONGVARCHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports LONGVARCHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == SQL_CVT_GUID); // Reference driver supports LONGVARCHAR to GUID conversion
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports LONGVARCHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports LONGVARCHAR to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports LONGVARCHAR to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports LONGVARCHAR to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports LONGVARCHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports LONGVARCHAR to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports LONGVARCHAR to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports LONGVARCHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports LONGVARCHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports LONGVARCHAR to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports LONGVARCHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_NUMERIC",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_NUMERIC, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports NUMERIC to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports NUMERIC to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports NUMERIC to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports NUMERIC to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports NUMERIC to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports NUMERIC to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports NUMERIC to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_REAL",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_REAL, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports REAL to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports REAL to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports REAL to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports REAL to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports REAL to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports REAL to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports REAL to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports REAL to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports REAL to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports REAL to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports REAL to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports REAL to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports REAL to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports REAL to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports REAL to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports REAL to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports REAL to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_SMALLINT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_SMALLINT, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports SMALLINT to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports SMALLINT to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports SMALLINT to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports SMALLINT to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports SMALLINT to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports SMALLINT to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports SMALLINT to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports SMALLINT to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports SMALLINT to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports SMALLINT to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports SMALLINT to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports SMALLINT to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports SMALLINT to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports SMALLINT to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports SMALLINT to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports SMALLINT to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports SMALLINT to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_TIME",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_TIME, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports TIME to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports TIME to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports TIME to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_TIMESTAMP",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_TIMESTAMP, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports TIMESTAMP to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports TIMESTAMP to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports TIMESTAMP to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports TIMESTAMP to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports TIMESTAMP to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_TINYINT",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_TINYINT, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports TINYINT to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports TINYINT to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports TINYINT to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports TINYINT to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports TINYINT to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports TINYINT to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports TINYINT to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports TINYINT to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports TINYINT to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports TINYINT to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports TINYINT to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports TINYINT to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports TINYINT to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports TINYINT to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports TINYINT to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports TINYINT to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports TINYINT to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_VARBINARY",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_VARBINARY, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports VARBINARY to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports VARBINARY to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == 0);
  REQUIRE((convert & SQL_CVT_DECIMAL) == 0);
  REQUIRE((convert & SQL_CVT_DOUBLE) == 0);
  REQUIRE((convert & SQL_CVT_FLOAT) == 0);
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == 0);
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == 0);
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == 0);
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports VARBINARY to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports VARBINARY to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_VARCHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_VARCHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == 0);
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports VARCHAR to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == 0);
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports VARCHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports VARCHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports VARCHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports VARCHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports VARCHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == 0);
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports VARCHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == 0);
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == 0);
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports VARCHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == 0);
  REQUIRE((convert & SQL_CVT_SMALLINT) == 0);
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports VARCHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports VARCHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == 0);
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports VARCHAR to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports VARCHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_WCHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_WCHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports WCHAR to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports WCHAR to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports WCHAR to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports WCHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports WCHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports WCHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports WCHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports WCHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == SQL_CVT_GUID); // Reference driver supports WCHAR to GUID conversion
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports WCHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports WCHAR to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports WCHAR to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == SQL_CVT_LONGVARBINARY); // Reference driver supports WCHAR to LONGVARBINARY conversion
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports WCHAR to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports WCHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports WCHAR to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports WCHAR to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports WCHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports WCHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports WCHAR to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports WCHAR to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports WCHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_WVARCHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_WVARCHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports WVARCHAR to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == SQL_CVT_BINARY); // Reference driver supports WVARCHAR to BINARY conversion
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports WVARCHAR to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports WVARCHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports WVARCHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports WVARCHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports WVARCHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports WVARCHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == SQL_CVT_GUID); // Reference driver supports WVARCHAR to GUID conversion
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports WVARCHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports WVARCHAR to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports WVARCHAR to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == SQL_CVT_LONGVARBINARY); // Reference driver supports WVARCHAR to LONGVARBINARY conversion
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports WVARCHAR to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports WVARCHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports WVARCHAR to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports WVARCHAR to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports WVARCHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports WVARCHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports WVARCHAR to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == SQL_CVT_VARBINARY); // Reference driver supports WVARCHAR to VARBINARY conversion
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports WVARCHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_CONVERT_WLONGVARCHAR",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER convert = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_CONVERT_WLONGVARCHAR, &convert, sizeof(convert), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((convert & SQL_CVT_BIGINT) == SQL_CVT_BIGINT); // Reference driver supports WLONGVARCHAR to BIGINT conversion
  REQUIRE((convert & SQL_CVT_BINARY) == 0);
  REQUIRE((convert & SQL_CVT_BIT) == SQL_CVT_BIT); // Reference driver supports WLONGVARCHAR to BIT conversion
  REQUIRE((convert & SQL_CVT_CHAR) == SQL_CVT_CHAR); // Reference driver supports WLONGVARCHAR to CHAR conversion
  REQUIRE((convert & SQL_CVT_DATE) == SQL_CVT_DATE); // Reference driver supports WLONGVARCHAR to DATE conversion
  REQUIRE((convert & SQL_CVT_DECIMAL) == SQL_CVT_DECIMAL); // Reference driver supports WLONGVARCHAR to DECIMAL conversion
  REQUIRE((convert & SQL_CVT_DOUBLE) == SQL_CVT_DOUBLE); // Reference driver supports WLONGVARCHAR to DOUBLE conversion
  REQUIRE((convert & SQL_CVT_FLOAT) == SQL_CVT_FLOAT); // Reference driver supports WLONGVARCHAR to FLOAT conversion
  REQUIRE((convert & SQL_CVT_GUID) == SQL_CVT_GUID); // Reference driver supports WLONGVARCHAR to GUID conversion
  REQUIRE((convert & SQL_CVT_INTEGER) == SQL_CVT_INTEGER); // Reference driver supports WLONGVARCHAR to INTEGER conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_YEAR_MONTH) == SQL_CVT_INTERVAL_YEAR_MONTH); // Reference driver supports WLONGVARCHAR to INTERVAL_YEAR_MONTH conversion
  REQUIRE((convert & SQL_CVT_INTERVAL_DAY_TIME) == SQL_CVT_INTERVAL_DAY_TIME); // Reference driver supports WLONGVARCHAR to INTERVAL_DAY_TIME conversion
  REQUIRE((convert & SQL_CVT_LONGVARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_LONGVARCHAR) == SQL_CVT_LONGVARCHAR); // Reference driver supports WLONGVARCHAR to LONGVARCHAR conversion
  REQUIRE((convert & SQL_CVT_NUMERIC) == SQL_CVT_NUMERIC); // Reference driver supports WLONGVARCHAR to NUMERIC conversion
  REQUIRE((convert & SQL_CVT_REAL) == SQL_CVT_REAL); // Reference driver supports WLONGVARCHAR to REAL conversion
  REQUIRE((convert & SQL_CVT_SMALLINT) == SQL_CVT_SMALLINT); // Reference driver supports WLONGVARCHAR to SMALLINT conversion
  REQUIRE((convert & SQL_CVT_TIME) == SQL_CVT_TIME); // Reference driver supports WLONGVARCHAR to TIME conversion
  REQUIRE((convert & SQL_CVT_TIMESTAMP) == SQL_CVT_TIMESTAMP); // Reference driver supports WLONGVARCHAR to TIMESTAMP conversion
  REQUIRE((convert & SQL_CVT_TINYINT) == SQL_CVT_TINYINT); // Reference driver supports WLONGVARCHAR to TINYINT conversion
  REQUIRE((convert & SQL_CVT_VARBINARY) == 0);
  REQUIRE((convert & SQL_CVT_VARCHAR) == SQL_CVT_VARCHAR); // Reference driver supports WLONGVARCHAR to VARCHAR conversion

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Deprecated ODBC 2.x Information Types.
// Drivers still need to support these for backward compatibility
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_FETCH_DIRECTION (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER fetchDir = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_FETCH_DIRECTION, &fetchDir, sizeof(fetchDir), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((fetchDir & SQL_FD_FETCH_NEXT) == SQL_FD_FETCH_NEXT); // Reference driver supports FETCH NEXT
  REQUIRE((fetchDir & SQL_FD_FETCH_FIRST) == 0);
  REQUIRE((fetchDir & SQL_FD_FETCH_LAST) == 0);
  REQUIRE((fetchDir & SQL_FD_FETCH_PRIOR) == 0);
  REQUIRE((fetchDir & SQL_FD_FETCH_ABSOLUTE) == 0);
  REQUIRE((fetchDir & SQL_FD_FETCH_RELATIVE) == 0);
  REQUIRE((fetchDir & SQL_FD_FETCH_BOOKMARK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_LOCK_TYPES (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER lockTypes = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_LOCK_TYPES, &lockTypes, sizeof(lockTypes), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((lockTypes & SQL_LCK_NO_CHANGE) == 0);
  REQUIRE((lockTypes & SQL_LCK_EXCLUSIVE) == SQL_LCK_EXCLUSIVE); // Reference driver supports exclusive locks
  REQUIRE((lockTypes & SQL_LCK_UNLOCK) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ODBC_API_CONFORMANCE (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT apiConf = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ODBC_API_CONFORMANCE, &apiConf, sizeof(apiConf), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(apiConf == SQL_OAC_LEVEL2); // Reference driver conforms to ODBC API Level 2

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_ODBC_SQL_CONFORMANCE (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT sqlConf = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_ODBC_SQL_CONFORMANCE, &sqlConf, sizeof(sqlConf), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sqlConf == SQL_OSC_CORE); // Reference driver conforms to ODBC SQL Core grammar

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_POS_OPERATIONS (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER posOps = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_POS_OPERATIONS, &posOps, sizeof(posOps), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support any positioned operations
  REQUIRE((posOps & SQL_POS_POSITION) == 0);
  REQUIRE((posOps & SQL_POS_REFRESH) == 0);
  REQUIRE((posOps & SQL_POS_UPDATE) == 0);
  REQUIRE((posOps & SQL_POS_DELETE) == 0);
  REQUIRE((posOps & SQL_POS_ADD) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_POSITIONED_STATEMENTS (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER posStmts = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_POSITIONED_STATEMENTS, &posStmts, sizeof(posStmts), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  // Reference driver does not support any positioned statements
  REQUIRE((posStmts & SQL_PS_POSITIONED_DELETE) == 0);
  REQUIRE((posStmts & SQL_PS_POSITIONED_UPDATE) == 0);
  REQUIRE((posStmts & SQL_PS_SELECT_FOR_UPDATE) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_SCROLL_CONCURRENCY (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER scrollConc = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_SCROLL_CONCURRENCY, &scrollConc, sizeof(scrollConc), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((scrollConc & SQL_SCCO_READ_ONLY) == SQL_SCCO_READ_ONLY); // Reference driver supports read-only cursors
  REQUIRE((scrollConc & SQL_SCCO_LOCK) == 0);
  REQUIRE((scrollConc & SQL_SCCO_OPT_ROWVER) == 0);
  REQUIRE((scrollConc & SQL_SCCO_OPT_VALUES) == 0);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: SQL_STATIC_SENSITIVITY (deprecated)",
                 "[odbc-api][getinfo][driver_info][deprecated]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER staticSens = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_STATIC_SENSITIVITY, &staticSens, sizeof(staticSens), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  
  REQUIRE((staticSens & SQL_SS_ADDITIONS) == SQL_SS_ADDITIONS); // Reference driver detects row additions
  REQUIRE((staticSens & SQL_SS_DELETIONS) == SQL_SS_DELETIONS); // Reference driver detects row deletions
  REQUIRE((staticSens & SQL_SS_UPDATES) == 0);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// Error Handling and Buffer Management Tests
// ============================================================================

TEST_CASE("SQLGetInfo: SQL_INVALID_HANDLE - NULL connection handle",
          "[odbc-api][getinfo][driver_info][error]") {
  char buffer[256];
  const SQLRETURN ret =
      SQLGetInfo(SQL_NULL_HDBC, SQL_DBMS_NAME, buffer, sizeof(buffer), nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: Returns SQL_SUCCESS_WITH_INFO when buffer too small",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char smallBuffer[4];
  SQLSMALLINT actualLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, smallBuffer, sizeof(smallBuffer), &actualLen);

  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(actualLen > sizeof(smallBuffer));

  const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE(std::string(records[0].sqlState) == "01004");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: Can query with NULL StringLengthPtr",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char driverName[256];
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, driverName, sizeof(driverName), nullptr);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(!std::string(driverName).empty());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: Can query just the length with NULL InfoValuePtr",
                 "[odbc-api][getinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT requiredLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, nullptr, 0, &requiredLen);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(requiredLen > 0);

  auto buffer = std::make_unique<char[]>(requiredLen + 1);
  SQLSMALLINT actualLen = 0;
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, buffer.get(), static_cast<SQLSMALLINT>(requiredLen + 1), &actualLen);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(actualLen == requiredLen);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: HY096/HY000 - Invalid InfoType",
                 "[odbc-api][getinfo][driver_info][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char buffer[256];
  ret = SQLGetInfo(dbc_handle(), 34463, buffer, sizeof(buffer), nullptr);

  // Note: Reference driver returns HY000 instead of HY096
  REQUIRE(ret == SQL_ERROR);
  const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE(std::string(records[0].sqlState) == "HY000");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetInfo: HY090 - Negative BufferLength",
                 "[odbc-api][getinfo][driver_info][error]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  char buffer[256];
  ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, buffer, -1, nullptr);

  REQUIRE_EXPECTED_ERROR(ret, "HY090", dbc_handle(), SQL_HANDLE_DBC);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcFixture, "SQLGetInfo: Requires active connection even for driver info",
                 "[odbc-api][getinfo][driver_info][error]") {
  char driverName[256];
  const SQLRETURN ret = SQLGetInfo(dbc_handle(), SQL_DRIVER_NAME, driverName, sizeof(driverName), nullptr);

  // Note: Reference driver requires active connection even for driver info
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLGetInfo: Returns error before connection for data source info",
                 "[odbc-api][getinfo][driver_info]") {
  char dbmsName[256];
  const SQLRETURN ret = SQLGetInfo(dbc_handle(), SQL_DBMS_NAME, dbmsName, sizeof(dbmsName), nullptr);

  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}
