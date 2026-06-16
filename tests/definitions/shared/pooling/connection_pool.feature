Feature: Connection pooling

  # ============================================================================
  # SNOWFLAKE POOLED CONNECTION (javax.sql)
  # Legacy: pooling.ConnectionPoolingDataSourceIT
  # ============================================================================

  @jdbc_e2e
  Scenario: should borrow logical connection and keep physical connection alive after close
    Given Snowflake connection pool data source is configured
    When A logical connection is borrowed and closed
    Then A new logical connection can run queries on the same physical connection

  @jdbc_e2e
  Scenario: should fire connection closed event when logical connection closes
    Given Snowflake connection pool data source is configured
    And A connection event listener is registered
    When A logical connection is borrowed and closed
    Then A connection closed event should be fired without error
    And The physical connection should remain open

  @jdbc_e2e
  Scenario: should fire connection error event when logical connection operation fails
    Given Snowflake connection pool data source is configured
    And A connection event listener is registered
    When An invalid catalog is set on a logical connection
    Then A connection error event should be fired with matching error code

  @jdbc_e2e
  Scenario: should get pooled connection with username and password
    Given Snowflake connection pool data source is configured
    When A pooled connection is obtained with username and password
    Then A query can be executed on the logical connection

  # ============================================================================
  # NODE.JS CONNECTION POOL PARITY (generic-pool semantics on logical connections)
  # Legacy reference: nodejs/tests/e2e/connection-pool.test.ts
  # ============================================================================

  @jdbc_e2e
  Scenario: should run concurrent queries on separate logical connections
    Given Snowflake connection pool data source is configured
    When Concurrent queries are executed on separate logical connections
    Then Each query should return its distinct value

  @jdbc_e2e
  Scenario: should propagate errors from logical connection operations
    Given Snowflake connection pool data source is configured
    When Invalid SQL is executed on a logical connection
    Then A SQL exception should be raised

  @jdbc_e2e
  Scenario: should reuse physical connection after logical close and reborrow
    Given Snowflake connection pool data source is configured
    When A logical connection is borrowed released and borrowed again
    Then The second logical connection should execute queries successfully
    And Both borrows should use the same underlying physical connection

  # ============================================================================
  # LOGICAL CONNECTION BEHAVIOR
  # Legacy: pooling.LogicalConnectionLatestIT, LogicalConnectionAlreadyClosedLatestIT,
  #         LogicalConnectionFeatureNotSupportedLatestIT
  # ============================================================================

  @jdbc_e2e
  Scenario: should reject operations on closed logical connection
    Given Snowflake connection pool data source is configured
    When A logical connection is closed
    Then Subsequent operations should raise connection closed error

  @jdbc_e2e
  Scenario: should reject unsupported features on logical connection
    Given Snowflake connection pool data source is configured
    When Unsupported JDBC features are invoked on a logical connection
    Then SQLFeatureNotSupportedException should be raised

  @jdbc_e2e
  Scenario: should create statement with holdability on logical connection
    Given Snowflake connection pool data source is configured
    When A statement is created with holdability on a logical connection
    Then The statement should execute and report expected holdability

  @jdbc_e2e
  Scenario Outline: should validate logical connection liveness with <timeout_label> timeout
    Given Snowflake connection pool data source is configured
    When Connection validity is checked with <timeout_seconds> second timeout on a logical connection
    Then Validation should <outcome>

    Examples:
      | timeout_label | timeout_seconds | outcome |
      | valid         | 10              | succeed |
      | negative      | -10             | fail    |

  @jdbc_e2e
  Scenario: should read client info from logical connection
    Given Snowflake connection pool data source is configured
    When Client info is read from a logical connection
    Then Client info should be empty and unknown keys return null

  @jdbc_e2e
  Scenario: should execute transaction statements on logical connection
    Given Snowflake connection pool data source is configured
    When Commit and rollback are used on a logical connection
    Then Transaction state should reflect DML changes correctly

  @jdbc_e2e
  Scenario: should set and read schema on logical connection
    Given Snowflake connection pool data source is configured
    When Schema is read and set on a logical connection
    Then Current schema should match database state

  @jdbc_e2e
  Scenario: should expose database metadata on logical connection
    Given Snowflake connection pool data source is configured
    When Database metadata is requested from a logical connection
    Then Product name should be Snowflake

  @jdbc_e2e
  Scenario: should unwrap logical connection to Snowflake connection implementation
    Given Snowflake connection pool data source is configured
    When Logical connection wrapper is inspected
    Then It should be a wrapper for SnowflakeConnectionImpl

  @jdbc_e2e
  Scenario: should execute prepared statement on logical connection
    Given Snowflake connection pool data source is configured
    When A prepared statement inserts rows on a logical connection
    Then Inserted rows should be readable

  @jdbc_e2e
  Scenario: should return native SQL unchanged on logical connection
    Given Snowflake connection pool data source is configured
    When nativeSQL is called on a logical connection
    Then The SQL text should be returned unchanged

  @jdbc_e2e
  Scenario: should query and set read-only state on logical connection
    Given Snowflake connection pool data source is configured
    When Read-only state is queried and set on a logical connection
    Then Read-only should remain false

  @jdbc_e2e
  Scenario: should return empty type map on logical connection
    Given Snowflake connection pool data source is configured
    When Type map is read from a logical connection
    Then An empty map should be returned

  @jdbc_e2e
  Scenario: should get and set network timeout on logical connection
    Given Snowflake connection pool data source is configured
    When Network timeout is read and updated on a logical connection
    Then Network timeout should reflect the configured value

  @jdbc_e2e
  Scenario: should close physical connection when logical connection is aborted
    Given Snowflake connection pool data source is configured
    When A logical connection is aborted
    Then The underlying physical connection should be closed

  @jdbc_e2e
  Scenario: should execute callable statement on logical connection
    Given Snowflake connection pool data source is configured
    When Callable statements are prepared and executed on a logical connection
    Then Stored procedure results and statement properties should match expectations

  @jdbc_e2e
  Scenario: should create and bind clob on logical connection
    Given Snowflake connection pool data source is configured
    When A clob value is created and inserted via prepared statement on a logical connection
    Then The inserted clob value should be readable

  @jdbc_e2e
  Scenario: should fire connection error event when physical connection delegates throw
    Given Snowflake connection pool data source is configured
    And A connection event listener is registered
    When Physical connection delegate operations throw SQLException on a logical connection
    Then Connection error event should be fired for each failing delegate operation

  # ============================================================================
  # THIRD-PARTY CONNECTION POOLS
  # Legacy: jdbc.ConnectionPoolingIT (HikariCP, C3P0, Apache DBCP)
  # Third-party pools use external pool managers with the Snowflake JDBC driver URL,
  # not SnowflakeConnectionPoolDataSource (javax.sql pooling API above).
  # thread_count matches legacy jdbc.ConnectionPoolingIT (10 concurrent workers per pool).
  # TODO: Add error-path scenarios for third-party pools:
  #   - pool exhausted when max connections are reached
  #   - worker query failure during concurrent borrow
  #   - connection refused or failed acquisition
  #   - connection borrow timeout when all connections are busy
  #   - invalid credentials rejected at pool initialisation
  #   - pool shutdown while connections are in use
  # ============================================================================

  @jdbc_e2e
  Scenario Outline: should query through <pool> connection pool with <thread_count> threads
    Given A temporary pooling query table exists
    When <thread_count> threads query through <pool> connection pool
    Then Each thread should read the expected row

    Examples:
      | pool   | thread_count |
      | Hikari | 10           |
      | C3P0   | 10           |
      | DBCP   | 10           |
