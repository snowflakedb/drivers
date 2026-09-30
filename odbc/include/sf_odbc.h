/*
 * sf_odbc.h — Public header for Snowflake-specific ODBC extensions.
 *
 * Defines the Snowflake vendor SQL data type codes and the custom connection
 * and statement attribute IDs.  Numeric values are kept in sync with the old
 * snowflake-odbc driver so that existing applications can migrate without
 * source changes.
 */

#ifndef SF_ODBC_H
#define SF_ODBC_H

/* -------------------------------------------------------------------------
 * Snowflake vendor SQL data type codes
 * -------------------------------------------------------------------------*/

#define SQL_SF_TIMESTAMP_LTZ 2000
#define SQL_SF_TIMESTAMP_TZ 2001
#define SQL_SF_TIMESTAMP_NTZ 2002
#define SQL_SF_ARRAY 2003
#define SQL_SF_OBJECT 2004
#define SQL_SF_VARIANT 2005

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

/* -------------------------------------------------------------------------
 * Snowflake-specific statement attributes
 * Base matches SQL_DRIVER_STMT_ATTR_BASE (0x4000) + 0x106, in sync with the
 * old snowflake-odbc driver's sf_odbc.h.
 * -------------------------------------------------------------------------*/
#ifndef SQL_DRIVER_STMT_ATTR_BASE
#define SQL_DRIVER_STMT_ATTR_BASE 0x00004000
#endif

#define SQL_SF_STMT_ATTR_BASE (SQL_DRIVER_STMT_ATTR_BASE + 0x106)

/* Query ID of the last executed statement (read-only string) */
#define SQL_SF_STMT_ATTR_LAST_QUERY_ID (SQL_SF_STMT_ATTR_BASE + 1)

/* Multi-statement execution count: -1 = auto, 0 = single, N = exact count */
#define SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT (SQL_SF_STMT_ATTR_BASE + 2)

#endif /* SF_ODBC_H */
