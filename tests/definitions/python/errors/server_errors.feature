@python
Feature: Server-side and network errors
  Errors triggered by server responses, network conditions, HTTP status codes,
  authentication flows, TLS/OCSP checks, and file transfer operations.
  HTTP status code scenarios require wiremock to simulate specific responses.

  # --- SQL Query Errors (real connection) ---

  @python_e2e
  Scenario: should raise DatabaseError for malformed SQL
    When The user executes "SELEC 1"
    Then DatabaseError is raised with errno 1003 and sqlstate "42000"

  @python_e2e
  Scenario: should raise DatabaseError for non-existent table
    When The user executes "SELECT * FROM nonexistent_table_<random>"
    Then DatabaseError is raised with errno 2003

  @python_e2e
  Scenario: should raise DatabaseError for non-existent database
    When The user executes "USE DATABASE nonexistent_db_<random>"
    Then DatabaseError is raised with errno 2043

  # --- Integrity Constraint Violations (real connection) ---

  @python_e2e
  Scenario: should raise IntegrityError for NULL in NOT NULL column
    Given A temporary table with schema "id INT NOT NULL, name VARCHAR NOT NULL"
    When The user executes "INSERT INTO t VALUES (1, null)"
    Then IntegrityError is raised with errno 100072

  @python_e2e
  Scenario: should succeed inserting valid values into NOT NULL columns
    Given A temporary table with schema "id INT NOT NULL, name VARCHAR NOT NULL"
    When The user executes "INSERT INTO t VALUES (1, 'Alice')"
    Then No error is raised and rowcount is 1

  # --- Authentication Errors (real connection) ---

  @python_e2e
  Scenario: should raise DatabaseError for invalid password
    When The user connects with an incorrect password
    Then DatabaseError is raised with errno 250001

  @python_e2e
  Scenario: should raise Error for non-existent account
    When The user connects with account "nonexistent_account_<random>"
    Then Error is raised with a non-default errno

  # --- HTTP Errors via Wiremock (query execution path) ---

  @python_e2e
  Scenario: should raise BadRequest when HTTP 400 exhausts retries
    Given Wiremock returns HTTP 400 for query requests
    When The client executes a query
    Then BadRequest is raised

  @python_e2e
  Scenario: should succeed when HTTP 400 is transient then 200
    Given Wiremock returns HTTP 400 once then HTTP 200
    When The client executes a query
    Then No error is raised

  @python_e2e
  Scenario: should raise InternalServerError when HTTP 500 exhausts retries
    Given Wiremock returns HTTP 500 for query requests
    When The client executes a query
    Then InternalServerError is raised

  @python_e2e
  Scenario: should raise BadGatewayError when HTTP 502 exhausts retries
    Given Wiremock returns HTTP 502 for query requests
    When The client executes a query
    Then BadGatewayError is raised

  @python_e2e
  Scenario: should raise OperationalError when HTTP 503 exhausts retries
    Given Wiremock returns HTTP 503 for query requests
    When The client executes a query
    Then OperationalError is raised

  @python_e2e
  Scenario: should raise GatewayTimeoutError for HTTP 504 during chunk download
    Given A result set where wiremock returns HTTP 504 for chunk downloads
    When The client fetches result rows
    Then GatewayTimeoutError is raised

  @python_e2e
  Scenario: should raise TooManyRequests when HTTP 429 exhausts retries
    Given Wiremock returns HTTP 429 for query requests
    When The client executes a query
    Then TooManyRequests is raised

  @python_e2e
  Scenario: should raise MethodNotAllowed for HTTP 405 during chunk download
    Given A result set where wiremock returns HTTP 405 for chunk downloads
    When The client fetches result rows
    Then MethodNotAllowed is raised

  @python_e2e
  Scenario: should succeed when HTTP 408 is transient then 200
    Given Wiremock returns HTTP 408 once then HTTP 200
    When The client executes a query
    Then No error is raised

  # --- HTTP Errors via Wiremock (authentication path) ---

  @python_e2e
  Scenario: should raise ForbiddenError for HTTP 403 on login
    Given Wiremock returns HTTP 403 for login requests
    When The client attempts to connect
    Then ForbiddenError is raised with message matching "Failed to connect to DB"

  @python_e2e
  Scenario: should raise BadGatewayError for HTTP 502 on login
    Given Wiremock returns HTTP 502 for login requests
    When The client attempts to connect
    Then BadGatewayError is raised with message matching "Service is unavailable"

  @python_e2e
  Scenario: should raise ServiceUnavailableError for HTTP 503 on login
    Given Wiremock returns HTTP 503 for login requests
    When The client attempts to connect
    Then ServiceUnavailableError is raised with message matching "Service is unavailable"

  # --- HTTP Errors via Wiremock (result set chunk downloads) ---

  @python_e2e
  Scenario: should raise InternalServerError for HTTP 500 during chunk download
    Given A result set where wiremock returns HTTP 500 for chunk downloads
    When The client fetches result rows
    Then InternalServerError is raised

  @python_e2e
  Scenario: should raise BadGatewayError for HTTP 502 during chunk download
    Given A result set where wiremock returns HTTP 502 for chunk downloads
    When The client fetches result rows
    Then BadGatewayError is raised

  @python_e2e
  Scenario: should raise TooManyRequests for HTTP 429 during chunk download
    Given A result set where wiremock returns HTTP 429 for chunk downloads
    When The client fetches result rows
    Then TooManyRequests is raised

  # --- File Transfer via Wiremock ---

  @python_e2e
  Scenario: should raise RequestExceedMaxRetryError when PUT upload exhausts storage retries
    Given Wiremock returns HTTP 503 for all storage PUT requests
    When The user executes a PUT command to an internal stage
    Then RequestExceedMaxRetryError is raised

  @python_e2e
  Scenario: should succeed PUT after storage token renewal
    Given Wiremock returns expired-token once then HTTP 200 for storage PUT
    When The user executes a PUT command to an internal stage
    Then The upload succeeds with no error

  @python_e2e
  Scenario: should fall back to inline binds when executemany stage creation fails
    Given Wiremock returns HTTP 403 for stage creation requests
    And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD is set to trigger optimization
    When executemany is called with a large parameter list
    Then The INSERT succeeds via inline binding fallback

  # --- Connection Timeout via Wiremock ---

  @python_e2e
  Scenario: should raise OperationalError for connection timeout during login
    Given Wiremock delays login response beyond the connection timeout
    When The client attempts to connect with a short timeout
    Then OperationalError is raised

  # --- Async Query (real connection) ---

  @python_e2e
  Scenario: should raise DatabaseError when retrieving results for failed async query
    Given An async query that will fail on the server
    When The user calls get_results_from_sfqid
    Then DatabaseError is raised
