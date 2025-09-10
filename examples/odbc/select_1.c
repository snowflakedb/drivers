#include <sql.h>
#include <sqlext.h>
#include <stdio.h>
#include <stdlib.h>

#include "macros.h"

void select_1(const char* connection_string) {
  SQLHENV env;
  SQLHDBC dbc;
  SQLHSTMT stmt;
  SQLRETURN ret;
  SQLCHAR outstr[1024];
  SQLSMALLINT outstrlen;

  // Allocate environment handle
  ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env));

  // Set ODBC version
  CHECK_ERROR(SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, (void*)SQL_OV_ODBC3, 0), SQL_HANDLE_ENV,
              env);

  // Allocate connection handle
  ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc));

  // Connect to the database
  CHECK_ERROR(SQLDriverConnect(dbc, NULL, (SQLCHAR*)connection_string, SQL_NTS, outstr,
                               sizeof(outstr), &outstrlen, SQL_DRIVER_NOPROMPT),
              SQL_HANDLE_DBC, dbc);

  // Allocate statement handle
  ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt));

  // Execute query
  CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)"SELECT 1", SQL_NTS), SQL_HANDLE_STMT, stmt);

  // Fetch data
  CHECK_ERROR(SQLFetch(stmt), SQL_HANDLE_STMT, stmt);

  // Get data
  SQLINTEGER result;
  CHECK_ERROR(SQLGetData(stmt, 1, SQL_C_LONG, &result, sizeof(result), NULL), SQL_HANDLE_STMT,
              stmt);
  printf("Result: %d\n", result);

  // Free statement handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));

  // Disconnect from the database
  ASSERT_SUCCESS(SQLDisconnect(dbc));

  // Free connection handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_DBC, dbc));

  // Free environment handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_ENV, env));
}
