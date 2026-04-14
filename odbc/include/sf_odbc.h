/*
 * sf_odbc.h — Public header for Snowflake-specific ODBC extensions.
 *
 * Defines custom connection attribute IDs for use with SQLSetConnectAttr /
 * SQLGetConnectAttr.  Numeric values are kept in sync with the old
 * snowflake-odbc driver so that existing applications can migrate without
 * source changes.
 */

#ifndef SF_ODBC_H
#define SF_ODBC_H

#ifndef SQL_DRIVER_CONN_ATTR_BASE
#define SQL_DRIVER_CONN_ATTR_BASE 0x00004000
#endif

#define SQL_SF_CONN_ATTR_BASE (SQL_DRIVER_CONN_ATTR_BASE + 0x53)

/* EVP_PKEY pointer — NOT supported in the new Rust driver (returns HYC00).
 * Use SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT or SQL_SF_CONN_ATTR_PRIV_KEY_BASE64
 * instead. */
#define SQL_SF_CONN_ATTR_PRIV_KEY (SQL_SF_CONN_ATTR_BASE + 1)

/* Application name */
#define SQL_SF_CONN_ATTR_APPLICATION (SQL_SF_CONN_ATTR_BASE + 2)

/* Private key as PEM string */
#define SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT (SQL_SF_CONN_ATTR_BASE + 3)

/* Private key password / passphrase */
#define SQL_SF_CONN_ATTR_PRIV_KEY_PASSWORD (SQL_SF_CONN_ATTR_BASE + 4)

/* Private key as base64-encoded string */
#define SQL_SF_CONN_ATTR_PRIV_KEY_BASE64 (SQL_SF_CONN_ATTR_BASE + 5)

/* Statement attribute base for Snowflake-specific statement attributes.
 * Uses SQL_DRIVER_STMT_ATTR_BASE from system ODBC headers (1000 on Windows, 16384 on iODBC).
 * Formula: (SQL_DRIVER_STMT_ATTR_BASE + 0x106).
 * The Rust implementation accepts both Windows (1263/1264) and Unix (16647/16648) values. */
#ifndef SQL_DRIVER_STMT_ATTR_BASE
#define SQL_DRIVER_STMT_ATTR_BASE 16384
#endif

#define SQL_SF_STMT_ATTR_BASE (SQL_DRIVER_STMT_ATTR_BASE + 0x106)

/* Last query ID — the ID of the most recently executed query on the statement.
 * Read-only string attribute. */
#define SQL_SF_STMT_ATTR_LAST_QUERY_ID (SQL_SF_STMT_ATTR_BASE + 1)

/* Multi-statement count — set via SQLSetStmtAttr before executing a
 * multi-statement SQL string.  Value is the number of semicolon-separated
 * statements in the batch. */
#define SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT (SQL_SF_STMT_ATTR_BASE + 2)

#endif /* SF_ODBC_H */
