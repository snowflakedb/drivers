#include <sql.h>
#include <sqlext.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "macros.h"

void put_file(const char* connection_string) {
  SQLHENV env;
  SQLHDBC dbc;
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

  // Create a temporary file for demonstration
  const char* temp_filename = "/tmp/test_put_file.csv";
  FILE* temp_file = fopen(temp_filename, "w");
  if (temp_file) {
    fprintf(temp_file, "col1,col2,col3\n1,2,3\n4,5,6\n");
    fclose(temp_file);
    printf("Created temporary file: %s\n", temp_filename);
  }

  {
    SQLHSTMT stmt;
    ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt));
    // Create a temporary stage
    const char* create_stage_sql = "CREATE OR REPLACE TEMPORARY STAGE EXAMPLE_PUT_STAGE";
    CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)create_stage_sql, SQL_NTS), SQL_HANDLE_STMT, stmt);
    printf("Created temporary stage: EXAMPLE_PUT_STAGE\n");
    ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));
  }

  // Execute PUT command to upload the file
  {
    SQLHSTMT stmt;
    ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt));
    char put_sql[512];
    snprintf(put_sql, sizeof(put_sql), "PUT 'file://%s' @EXAMPLE_PUT_STAGE", temp_filename);
    CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)put_sql, SQL_NTS), SQL_HANDLE_STMT, stmt);
    CHECK_ERROR(SQLFetch(stmt), SQL_HANDLE_STMT, stmt);
    SQLCHAR source[256], target[256], status[64], message[512];
    SQLINTEGER source_size, target_size;
    SQLCHAR source_compression[64], target_compression[64];

    CHECK_ERROR(SQLGetData(stmt, 1, SQL_C_CHAR, source, sizeof(source), NULL), SQL_HANDLE_STMT,
                stmt);
    CHECK_ERROR(SQLGetData(stmt, 2, SQL_C_CHAR, target, sizeof(target), NULL), SQL_HANDLE_STMT,
                stmt);
    CHECK_ERROR(SQLGetData(stmt, 3, SQL_C_LONG, &source_size, sizeof(source_size), NULL),
                SQL_HANDLE_STMT, stmt);
    CHECK_ERROR(SQLGetData(stmt, 4, SQL_C_LONG, &target_size, sizeof(target_size), NULL),
                SQL_HANDLE_STMT, stmt);
    CHECK_ERROR(
        SQLGetData(stmt, 5, SQL_C_CHAR, source_compression, sizeof(source_compression), NULL),
        SQL_HANDLE_STMT, stmt);
    CHECK_ERROR(
        SQLGetData(stmt, 6, SQL_C_CHAR, target_compression, sizeof(target_compression), NULL),
        SQL_HANDLE_STMT, stmt);
    CHECK_ERROR(SQLGetData(stmt, 7, SQL_C_CHAR, status, sizeof(status), NULL), SQL_HANDLE_STMT,
                stmt);
    CHECK_ERROR(SQLGetData(stmt, 8, SQL_C_CHAR, message, sizeof(message), NULL), SQL_HANDLE_STMT,
                stmt);

    printf("PUT Result:\n");
    printf("  Source: %s\n", source);
    printf("  Target: %s\n", target);
    printf("  Source Size: %d bytes\n", source_size);
    printf("  Target Size: %d bytes\n", target_size);
    printf("  Source Compression: %s\n", source_compression);
    printf("  Target Compression: %s\n", target_compression);
    printf("  Status: %s\n", status);
    printf("  Message: %s\n", message);
    ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));
  }

  exit(1);

  {
    SQLHSTMT stmt;
    ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt));
    CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)"LS @EXAMPLE_PUT_STAGE", SQL_NTS), SQL_HANDLE_STMT,
                stmt);
    printf("\nStage contents:\n");

    while (SQLFetch(stmt) == SQL_SUCCESS) {
      SQLCHAR filename[256];
      SQLINTEGER file_size;
      CHECK_ERROR(SQLGetData(stmt, 1, SQL_C_CHAR, filename, sizeof(filename), NULL),
                  SQL_HANDLE_STMT, stmt);
      CHECK_ERROR(SQLGetData(stmt, 2, SQL_C_LONG, &file_size, sizeof(file_size), NULL),
                  SQL_HANDLE_STMT, stmt);
      printf("  File: %s (Size: %d bytes)\n", filename, file_size);
    }
    ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));
  }

  // Clean up temporary file
  remove(temp_filename);
  printf("\nTemporary file removed\n");
}
