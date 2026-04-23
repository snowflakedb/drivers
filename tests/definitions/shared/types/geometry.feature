@python
Feature: GEOMETRY type support
  # Snowflake GEOMETRY type represents geospatial data in a planar coordinate system.
  # Values are returned as JSON strings (GeoJSON format by default).
  # Input via WKT strings through TO_GEOMETRY().
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast geometry values to appropriate type
    # Python: Values should be cast to 'str' type (GeoJSON string)
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
    Then All values should be returned as appropriate type

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geometry literals with different shapes
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY('POINT(0 0)'), TO_GEOMETRY('LINESTRING(1 1, 2 2, 3 3)'), TO_GEOMETRY('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))')" is executed
    Then Result should contain GeoJSON values for Point, LineString, and Polygon

  # =========================================================================== #
  #                             NULL handling                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should handle NULL geometry values from literals
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY('POINT(0 0)'), TO_GEOMETRY(NULL)" is executed
    Then Result should contain [GeoJSON Point, NULL]

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geometry values from table
    Given Snowflake client is logged in
    And Table with GEOMETRY column exists with WKT values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain the expected GeoJSON values

  @python_e2e
  Scenario: should handle NULL geometry values from table
    Given Snowflake client is logged in
    And Table with GEOMETRY column exists containing NULLs and values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain [GeoJSON Point, NULL]

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download geometry data in multiple chunks
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY('POINT(' || seq8() || ' ' || seq8() || ')') AS geo FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
    Then All 20000 rows should be fetched and each should be a non-null string value

  # =========================================================================== #
  #                           Parameter binding                                 #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geometry using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY(?)" is executed with bound WKT string 'POINT(1820.12 890.56)'
    Then Result should contain a GeoJSON Point value

  @python_e2e
  Scenario: should select NULL geometry using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY(?)" is executed with bound NULL value
    Then Result should be NULL

  @python_e2e
  Scenario: should insert geometry using parameter binding
    Given Snowflake client is logged in
    And Table with GEOMETRY column exists
    When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
    Then SELECT should return the inserted GeoJSON values

  # =========================================================================== #
  #                         JSON result format                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select geometry with JSON result format
    Given Snowflake client is logged in
    And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
    When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
    Then Result should contain a GeoJSON Point value
