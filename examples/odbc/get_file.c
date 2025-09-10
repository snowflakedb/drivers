#include <sql.h>
#include <sqlext.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "macros.h"

void get_file(const char* connection_string) {
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

  // Create a temporary file for demonstration and upload it first
  const char* temp_filename = "/tmp/test_get_file.csv";
  FILE* temp_file = fopen(temp_filename, "w");
  if (temp_file) {
    fprintf(temp_file, "id,name,value\n1,alice,100\n2,bob,200\n3,charlie,300\n");
    fclose(temp_file);
    printf("Created temporary file for upload: %s\n", temp_filename);
  } else {
    printf("Failed to create temporary file\n");
    goto cleanup;
  }

  // Create a temporary stage
  const char* create_stage_sql = "CREATE OR REPLACE TEMPORARY STAGE EXAMPLE_GET_STAGE";
  CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)create_stage_sql, SQL_NTS), SQL_HANDLE_STMT, stmt);
  printf("Created temporary stage: EXAMPLE_GET_STAGE\n");

  // First, upload a file to demonstrate GET
  char put_sql[512];
  snprintf(put_sql, sizeof(put_sql), "PUT 'file://%s' @EXAMPLE_GET_STAGE", temp_filename);
  CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)put_sql, SQL_NTS), SQL_HANDLE_STMT, stmt);

  // Consume the PUT result
  if (SQLFetch(stmt) == SQL_SUCCESS) {
    SQLCHAR status[64];
    CHECK_ERROR(SQLGetData(stmt, 7, SQL_C_CHAR, status, sizeof(status), NULL), SQL_HANDLE_STMT,
                stmt);
    printf("File uploaded with status: %s\n", status);
  }

  // Free and reallocate statement handle for GET operation
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));
  ASSERT_SUCCESS(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt));

  // Create download directory
  const char* download_dir = "/tmp/download_test";
  char mkdir_cmd[256];
  snprintf(mkdir_cmd, sizeof(mkdir_cmd), "mkdir -p %s", download_dir);
  system(mkdir_cmd);
  printf("Created download directory: %s\n", download_dir);

  // Execute GET command to download the file
  char get_sql[512];
  snprintf(get_sql, sizeof(get_sql), "GET @EXAMPLE_GET_STAGE/test_get_file.csv 'file://%s/'",
           download_dir);
  CHECK_ERROR(SQLExecDirect(stmt, (SQLCHAR*)get_sql, SQL_NTS), SQL_HANDLE_STMT, stmt);

  // Fetch and display GET result
  CHECK_ERROR(SQLFetch(stmt), SQL_HANDLE_STMT, stmt);

  // Get GET result columns (file, size, status, message)
  SQLCHAR file[256], status[64], message[512];
  SQLINTEGER file_size;

  CHECK_ERROR(SQLGetData(stmt, 1, SQL_C_CHAR, file, sizeof(file), NULL), SQL_HANDLE_STMT, stmt);
  CHECK_ERROR(SQLGetData(stmt, 2, SQL_C_LONG, &file_size, sizeof(file_size), NULL), SQL_HANDLE_STMT,
              stmt);
  CHECK_ERROR(SQLGetData(stmt, 3, SQL_C_CHAR, status, sizeof(status), NULL), SQL_HANDLE_STMT, stmt);
  CHECK_ERROR(SQLGetData(stmt, 4, SQL_C_CHAR, message, sizeof(message), NULL), SQL_HANDLE_STMT,
              stmt);

  printf("\nGET Result:\n");
  printf("  File: %s\n", file);
  printf("  Size: %d bytes\n", file_size);
  printf("  Status: %s\n", status);
  printf("  Message: %s\n", message);

  // Verify the downloaded file exists
  char downloaded_file_path[512];
  snprintf(downloaded_file_path, sizeof(downloaded_file_path), "%s/test_get_file.csv.gz",
           download_dir);

  if (access(downloaded_file_path, F_OK) == 0) {
    printf("\nSuccessfully downloaded file to: %s\n", downloaded_file_path);

    // Display file size
    FILE* downloaded_file = fopen(downloaded_file_path, "rb");
    if (downloaded_file) {
      fseek(downloaded_file, 0, SEEK_END);
      long actual_size = ftell(downloaded_file);
      fclose(downloaded_file);
      printf("Actual downloaded file size: %ld bytes\n", actual_size);
    }
  } else {
    printf("Warning: Downloaded file not found at expected location: %s\n", downloaded_file_path);
  }

  // Clean up files and directories
  remove(temp_filename);
  snprintf(mkdir_cmd, sizeof(mkdir_cmd), "rm -rf %s", download_dir);
  system(mkdir_cmd);
  printf("\nTemporary files and directories cleaned up\n");

cleanup:
  // Free statement handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_STMT, stmt));

  // Disconnect from the database
  ASSERT_SUCCESS(SQLDisconnect(dbc));

  // Free connection handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_DBC, dbc));

  // Free environment handle
  ASSERT_SUCCESS(SQLFreeHandle(SQL_HANDLE_ENV, env));
}
