@python @core_not_needed
Feature: GEOMETRY type support
  # Snowflake GEOMETRY type represents geospatial data in a planar coordinate system.
  # Values are returned as strings by default (GeoJSON format).
  # The output format is controlled by the GEOMETRY_OUTPUT_FORMAT session parameter:
  #   GeoJSON (default), WKT, EWKT -> VARCHAR (str in Python)
  #   WKB, EWKB -> BINARY (bytes in Python)
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
  Scenario Outline: should select <shape> geometry literal
    Given Snowflake client is logged in
    When Query "SELECT <query_value>" is executed
    Then Result should contain a GeoJSON <shape> value

    Examples:
      | shape      | query_value                                              |
      | Point      | TO_GEOMETRY('POINT(0 0)')                                |
      | LineString | TO_GEOMETRY('LINESTRING(1 1, 2 2, 3 3)')                 |
      | Polygon    | TO_GEOMETRY('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))')       |

  # =========================================================================== #
  #                          Output format handling                             #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select geometry in <format> output format
    Given Snowflake client is logged in
    And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
    When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
    Then Result should be returned as <expected_type> type

    Examples:
      | format  | expected_type |
      | GeoJSON | str           |
      | WKT     | str           |
      | WKB     | bytes         |
      | EWKT    | str           |
      | EWKB    | bytes         |

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
    # skip_for_json_result_set
    Given Snowflake client is logged in
    When Query generating 20000 geometry points is executed
    Then All 20000 rows should be fetched and each should be a non-null string value

  # =========================================================================== #
  #                           Parameter binding                                 #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select geometry using parameter binding with <input_type> value
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
    Then Result should <expected_result>

    Examples:
      | input_type | expected_result               |
      | WKT string | contain a GeoJSON Point value |
      | NULL       | be NULL                       |

  @python_e2e
  Scenario: should insert geometry using parameter binding
    Given Snowflake client is logged in
    And Table with GEOMETRY column exists
    When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
    Then SELECT should return the inserted GeoJSON values
