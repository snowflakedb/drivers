@python
Feature: GEOGRAPHY type support
  # Snowflake GEOGRAPHY type represents geospatial data on a sphere (WGS84).
  # Values are returned as JSON strings (GeoJSON format by default).
  # Input via WKT strings or GeoJSON through TO_GEOGRAPHY().
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast geography values to appropriate type
    # Python: Values should be cast to 'str' type (GeoJSON string)
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
    Then All values should be returned as appropriate type

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geography literals with different shapes
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)'), TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')" is executed
    Then Result should contain GeoJSON values for Point, LineString, and Polygon

  @python_e2e
  Scenario: should select geography from GeoJSON input
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('{\"type\":\"Point\",\"coordinates\":[-122.35,37.55]}')" is executed
    Then Result should contain a GeoJSON Point value

  # =========================================================================== #
  #                             NULL handling                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should handle NULL geography values from literals
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)'), TO_GEOGRAPHY(NULL)" is executed
    Then Result should contain [GeoJSON Point, NULL]

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geography values from table
    Given Snowflake client is logged in
    And Table with GEOGRAPHY column exists with WKT values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain the expected GeoJSON values

  @python_e2e
  Scenario: should handle NULL geography values from table
    Given Snowflake client is logged in
    And Table with GEOGRAPHY column exists containing NULLs and values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain [GeoJSON Point, NULL]

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download geography data in multiple chunks
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('POINT(' || (MOD(seq8(), 360) - 180) || ' ' || (MOD(seq8(), 180) - 90) || ')') AS geo FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
    Then All 20000 rows should be fetched and each should be a non-null string value

  # =========================================================================== #
  #                           Parameter binding                                 #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geography using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound WKT string 'POINT(-122.35 37.55)'
    Then Result should contain a GeoJSON Point value

  @python_e2e
  Scenario: should select NULL geography using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound NULL value
    Then Result should be NULL

  @python_e2e
  Scenario: should insert geography using parameter binding
    Given Snowflake client is logged in
    And Table with GEOGRAPHY column exists
    When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
    Then SELECT should return the inserted GeoJSON values

  # =========================================================================== #
  #                         JSON result format                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geography with JSON result format
    Given Snowflake client is logged in
    And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
    When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
    Then Result should contain a GeoJSON Point value
