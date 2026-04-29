@python @core_not_needed
Feature: GEOGRAPHY type support
  # Snowflake GEOGRAPHY type represents geospatial data on a sphere (WGS84).
  # Values are returned as strings by default (GeoJSON format).
  # The output format is controlled by the GEOGRAPHY_OUTPUT_FORMAT session parameter:
  #   GeoJSON (default), WKT, EWKT -> VARCHAR (str in Python)
  #   WKB, EWKB -> BINARY (bytearray in Python)
  # Input via WKT strings or GeoJSON through TO_GEOGRAPHY().
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-geospatial

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select <shape> geography literal
    Given Snowflake client is logged in
    When Query "SELECT <query_value>" is executed
    Then Result should contain a GeoJSON <shape> value

    Examples:
      | shape      | query_value                                                |
      | Point      | TO_GEOGRAPHY('POINT(-122.35 37.55)')                       |
      | LineString | TO_GEOGRAPHY('LINESTRING(0 0, 1 1, 2 2)')                  |
      | Polygon    | TO_GEOGRAPHY('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')    |

  @python_e2e
  Scenario: should select geography from GeoJSON input
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is executed
    Then Result should contain a GeoJSON Point value

  # =========================================================================== #
  #                     Type casting per output format                          #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should cast geography to <expected_type> for <format> output format
    Given Snowflake client is logged in
    And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
    When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
    Then Result should be returned as <expected_type> type

    Examples:
      | format  | expected_type |
      | GeoJSON | str           |
      | WKT     | str           |
      | WKB     | bytearray     |
      | EWKT    | str           |
      | EWKB    | bytearray     |

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
    # skip_for_json_result_set
    Given Snowflake client is logged in
    When Query generating 20000 geography points is executed
    Then All 20000 rows should be fetched with valid GeoJSON Point values

  # =========================================================================== #
  #                           Parameter binding                                 #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select geography using parameter binding with <input_type> value
    Given Snowflake client is logged in
    When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
    Then Result should <expected_result>

    Examples:
      | input_type | expected_result               |
      | WKT string | contain a GeoJSON Point value |
      | NULL       | be NULL                       |

  @python_e2e
  Scenario: should insert geography using parameter binding
    Given Snowflake client is logged in
    And Table with GEOGRAPHY column exists
    When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
    Then SELECT should return the inserted GeoJSON values
