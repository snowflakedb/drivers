
enum StatusCode {
  OK = 0,
  UNKNOWN = 1,
  NOT_IMPLEMENTED = 2,
  NOT_FOUND = 3,
  ALREADY_EXISTS = 4,
  INVALID_ARGUMENT = 5,
  INVALID_STATE = 6,
  INVALID_DATA = 7,
  IO = 8,
  CANCELLED = 9,
  UNAUTHENTICATED = 10,
  UNAUTHORIZED = 11
}

enum InfoCode {
  VENDOR_NAME = 0,
  VENDOR_VERSION = 1,
  VENDOR_ARROW_VERSION = 2,
  VENDOR_SQL = 100,
  VENDOR_SUBSTRAIT = 101,
  VENDOR_SUBSTRAIT_MIN_VERSION = 102,
  VENDOR_SUBSTRAIT_MAX_VERSION = 103,
  DRIVER_NAME = 200,
  DRIVER_VERSION = 201,
  DRIVER_ARROW_VERSION = 202,
  DRIVER_ADBC_VERSION = 203
}

struct ErrorDetail {
  1: required string key;
  2: required string value;
}

exception DriverException {
  1: required string message;
  2: required StatusCode status_code;
  3: optional i32 vendor_code;
  4: optional string sqlstate;
  5: optional list<ErrorDetail> details;
}

struct ExecuteResult {
  1: required ArrowArrayStreamPtr stream;
  2: required i64 rows_affected;
}

struct PartitionedResult {
  1: required i64 schema;
  2: required list<binary> partitions;
  3: required i64 rows_affected;
}

struct DatabaseHandle {
  1: required i64 id;
  2: required i64 magic;
}

struct ConnectionHandle {
  1: required i64 id;
  2: required i64 magic;
}

struct StatementHandle {
  1: required i64 id;
  2: required i64 magic;
}

struct ArrowArrayStreamPtr {
  1: required binary value;
}

struct ArrowSchemaPtr {
  1: required binary value;
}

struct ArrowArrayPtr {
  1: required binary value;
}

service DatabaseDriver {

  /**
   * Create a new, uninitialized database object.
   * Corresponds to AdbcDatabaseNew.
   * @return An opaque handle to the server-side database object.
   */
  DatabaseHandle databaseNew() throws (1: DriverException e);

  /**
   * Set a string-valued option for a database.
   * Corresponds to AdbcDatabaseSetOption.
   */
  void databaseSetOptionString(1: DatabaseHandle db_handle, 2: string key, 3: string value) throws (1: DriverException e);

  /**
   * Set a byte-valued option for a database.
   * Corresponds to AdbcDatabaseSetOptionBytes.
   */
  void databaseSetOptionBytes(1: DatabaseHandle db_handle, 2: string key, 3: binary value) throws (1: DriverException e);

  /**
   * Set an integer-valued option for a database.
   * Corresponds to AdbcDatabaseSetOptionInt.
   */
  void databaseSetOptionInt(1: DatabaseHandle db_handle, 2: string key, 3: i64 value) throws (1: DriverException e);

  /**
   * Set a double-valued option for a database.
   * Corresponds to AdbcDatabaseSetOptionDouble.
   */
  void databaseSetOptionDouble(1: DatabaseHandle db_handle, 2: string key, 3: double value) throws (1: DriverException e);

  /**
   * Finalize database initialization.
   * Corresponds to AdbcDatabaseInit.
   */
  void databaseInit(1: DatabaseHandle db_handle) throws (1: DriverException e);

  /**
   * Release the database object and its resources. The handle is invalidated.
   * Corresponds to AdbcDatabaseRelease.
   */
  void databaseRelease(1: DatabaseHandle db_handle) throws (1: DriverException e);

  /**
   * Create a new, uninitialized connection object.
   * Corresponds to AdbcConnectionNew.
   * @return An opaque handle to the server-side connection object.
   */
  ConnectionHandle connectionNew() throws (1: DriverException e);

  /**
   * Set a string-valued option for a connection.
   * Corresponds to AdbcConnectionSetOption.
   */
  void connectionSetOptionString(1: ConnectionHandle conn_handle, 2: string key, 3: string value) throws (1: DriverException e);

  /**
   * Set a byte-valued option for a connection.
   * Corresponds to AdbcConnectionSetOptionBytes.
   */
  void connectionSetOptionBytes(1: ConnectionHandle conn_handle, 2: string key, 3: binary value) throws (1: DriverException e);

  /**
   * Set an integer-valued option for a connection.
   * Corresponds to AdbcConnectionSetOptionInt.
   */
  void connectionSetOptionInt(1: ConnectionHandle conn_handle, 2: string key, 3: i64 value) throws (1: DriverException e);

  /**
   * Set a double-valued option for a connection.
   * Corresponds to AdbcConnectionSetOptionDouble.
   */
  void connectionSetOptionDouble(1: ConnectionHandle conn_handle, 2: string key, 3: double value) throws (1: DriverException e);

  /**
   * Finalize connection initialization.
   * Corresponds to AdbcConnectionInit.
   */
  void connectionInit(1: ConnectionHandle conn_handle, 2: DatabaseHandle db_handle) throws (1: DriverException e);

  /**
   * Release the connection object and its resources. The handle is invalidated.
   * Corresponds to AdbcConnectionRelease.
   */
  void connectionRelease(1: ConnectionHandle conn_handle) throws (1: DriverException e);

  /**
   * Get metadata about the database/driver.
   * Corresponds to AdbcConnectionGetInfo.
   * @param info_codes A list of codes for the metadata to retrieve. If null/empty, retrieve all.
   * @return An Arrow IPC stream containing the metadata.
   */
  binary connectionGetInfo(1: ConnectionHandle conn_handle, 2: optional list<InfoCode> info_codes) throws (1: DriverException e);

  /**
   * Get a hierarchical view of catalogs, DB schemas, tables, and columns.
   * Corresponds to AdbcConnectionGetObjects.
   * @return An Arrow IPC stream containing the metadata.
   */
  binary connectionGetObjects(1: ConnectionHandle conn_handle, 2: i32 depth, 3: optional string catalog, 4: optional string db_schema, 5: optional string table_name, 6: optional list<string> table_type, 7: optional string column_name) throws (1: DriverException e);

  /**
   * Get the Arrow schema of a table.
   * Corresponds to AdbcConnectionGetTableSchema.
   * @return A serialized ArrowSchema in IPC format.
   */
  binary connectionGetTableSchema(1: ConnectionHandle conn_handle, 2: optional string catalog, 3: optional string db_schema, 4: string table_name) throws (1: DriverException e);

  /**
   * Get a list of table types in the database.
   * Corresponds to AdbcConnectionGetTableTypes.
   * @return An Arrow IPC stream containing the table types.
   */
  binary connectionGetTableTypes(1: ConnectionHandle conn_handle) throws (1: DriverException e);

  /**
   * Commit any pending transactions.
   * Corresponds to AdbcConnectionCommit.
   */
  void connectionCommit(1: ConnectionHandle conn_handle) throws (1: DriverException e);

  /**
   * Roll back any pending transactions.
   * Corresponds to AdbcConnectionRollback.
   */
  void connectionRollback(1: ConnectionHandle conn_handle) throws (1: DriverException e);


  // --------------------------------------------------------------------------
  //  Statement Operations
  // --------------------------------------------------------------------------

  /**
   * Create a new statement.
   * Corresponds to AdbcStatementNew.
   * @return An opaque handle to the server-side statement object.
   */
  StatementHandle statementNew(1: ConnectionHandle conn_handle) throws (1: DriverException e);

  /**
   * Release the statement object.
   * Corresponds to AdbcStatementRelease.
   */
  void statementRelease(1: StatementHandle stmt_handle) throws (1: DriverException e);

  /**
   * Set the SQL query to execute.
   * Corresponds to AdbcStatementSetSqlQuery.
   */
  void statementSetSqlQuery(1: StatementHandle stmt_handle, 2: string query) throws (1: DriverException e);

  /**
   * Set the Substrait plan to execute.
   * Corresponds to AdbcStatementSetSubstraitPlan.
   */
  void statementSetSubstraitPlan(1: StatementHandle stmt_handle, 2: binary plan) throws (1: DriverException e);

  /**
   * Prepare a statement for execution.
   * Corresponds to AdbcStatementPrepare.
   */
  void statementPrepare(1: StatementHandle stmt_handle) throws (1: DriverException e);

  /**
   * Set a string-valued option for a statement.
   * Corresponds to AdbcStatementSetOption.
   */
  void statementSetOptionString(1: StatementHandle stmt_handle, 2: string key, 3: string value) throws (1: DriverException e);

  /**
   * Set a byte-valued option for a statement.
   * Corresponds to AdbcStatementSetOptionBytes.
   */
  void statementSetOptionBytes(1: StatementHandle stmt_handle, 2: string key, 3: binary value) throws (1: DriverException e);

  /**
   * Set an integer-valued option for a statement.
   * Corresponds to AdbcStatementSetOptionInt.
   */
  void statementSetOptionInt(1: StatementHandle stmt_handle, 2: string key, 3: i64 value) throws (1: DriverException e);

  /**
   * Set a double-valued option for a statement.
   * Corresponds to AdbcStatementSetOptionDouble.
   */
  void statementSetOptionDouble(1: StatementHandle stmt_handle, 2: string key, 3: double value) throws (1: DriverException e);

  /**
   * Get the schema for the parameters of a prepared statement.
   * Corresponds to AdbcStatementGetParameterSchema.
   * @return A serialized ArrowSchema in IPC format.
   */
  ArrowSchemaPtr statementGetParameterSchema(1: StatementHandle stmt_handle) throws (1: DriverException e);

  /**
   * Bind a single batch of values to a prepared statement.
   * Corresponds to AdbcStatementBind.
   * @param values An Arrow RecordBatch serialized in IPC format.
   */
  void statementBind(1: StatementHandle stmt_handle, 2: ArrowSchemaPtr schema, 3: ArrowArrayPtr array) throws (1: DriverException e);

  /**
   * Bind a stream of values to a statement (for bulk ingestion).
   * Corresponds to AdbcStatementBindStream.
   * @param stream An Arrow stream serialized in IPC format.
   */
  void statementBindStream(1: StatementHandle stmt_handle, 2: binary stream) throws (1: DriverException e);

  /**
   * Execute a query or a statement with bound data.
   * Corresponds to AdbcStatementExecuteQuery.
   * @return An ExecuteResult struct containing the result stream and rows affected.
   */
  ExecuteResult statementExecuteQuery(1: StatementHandle stmt_handle) throws (1: DriverException e);

  /**
   * Execute a query and get a description of the result partitions.
   * Corresponds to AdbcStatementExecutePartitions.
   * @return A PartitionedResult struct containing schema and partition descriptors.
   */
  PartitionedResult statementExecutePartitions(1: StatementHandle stmt_handle) throws (1: DriverException e);

  /**
   * Read a single partition of a result set.
   * Corresponds to AdbcConnectionReadPartition.
   * @param partition_descriptor An opaque descriptor from statementExecutePartitions.
   * @return An Arrow IPC stream for the requested partition.
   */
  i64 statementReadPartition(1: StatementHandle stmt_handle, 2: binary partition_descriptor) throws (1: DriverException e);
}
