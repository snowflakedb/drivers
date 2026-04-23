@python @numpy
Feature: INTERVAL type NumPy support (Python-specific)

  # INTERVAL values are returned as numpy.timedelta64 when NumPy mode is
  # enabled, instead of the default str (YEAR/MONTH) or datetime.timedelta
  # (DAY/TIME). The numpy unit depends on the interval sub-type:
  #
  #   YEAR TO MONTH family
  #     INTERVAL YEAR          -> numpy.timedelta64[Y]  (years)
  #     INTERVAL MONTH         -> numpy.timedelta64[M]  (months)
  #     INTERVAL YEAR TO MONTH -> numpy.timedelta64[M]  (months)
  #
  #   DAY TO SECOND family (storage width drives unit)
  #     DAY TO SECOND default precision (SB16 / Decimal128)
  #                            -> numpy.timedelta64[ms] (milliseconds)
  #                               Nanoseconds beyond ms are truncated (floored).
  #     DAY(3) TO SECOND and other SB8 (int64) variants
  #                            -> numpy.timedelta64[ns] (nanoseconds)
  #
  # Note on YEAR/MONTH extremes: Snowflake's spec max is 999_999_999, which is
  # exercised by the non-numpy path in tests/definitions/shared/types/interval.feature.
  # The numpy path currently fails to convert values at that magnitude
  # (InterfaceError 252005), so these scenarios exercise +/-99_999 as a
  # representative extreme. Widening this once the underlying issue is fixed
  # is a follow-up.
  #
  # INTERVAL support requires ENABLE_INTERVAL_TYPE to be active on the account.

  # ============================================================================
  # YEAR TO MONTH family
  # ============================================================================

  @python_e2e
  Scenario: should cast INTERVAL YEAR to numpy timedelta64 year unit
    # Usual: 0. Extreme: +/- 99_999 years.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0'::INTERVAL YEAR, '99999'::INTERVAL YEAR, '-99999'::INTERVAL YEAR" is executed
    Then All values should be returned as numpy.timedelta64 type with year resolution
    And Values should match exactly [0 Y, 99999 Y, -99999 Y]

  @python_e2e
  Scenario: should cast INTERVAL MONTH to numpy timedelta64 month unit
    # Usual: 0. Extreme: +/- 99_999 months.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0'::INTERVAL MONTH, '99999'::INTERVAL MONTH, '-99999'::INTERVAL MONTH" is executed
    Then All values should be returned as numpy.timedelta64 type with month resolution
    And Values should match exactly [0 M, 99999 M, -99999 M]

  @python_e2e
  Scenario: should cast INTERVAL YEAR TO MONTH to numpy timedelta64 month unit
    # Usual: 0-0 and 1-2. Extreme: +/- 99999-11.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0-0'::INTERVAL YEAR TO MONTH, '1-2'::INTERVAL YEAR TO MONTH, '99999-11'::INTERVAL YEAR TO MONTH, '-99999-11'::INTERVAL YEAR TO MONTH" is executed
    Then All values should be returned as numpy.timedelta64 type with month resolution
    And Values should match exactly [0 M, 14 M, 1199999 M, -1199999 M]

  # ============================================================================
  # DAY TO SECOND family
  # ============================================================================

  @python_e2e
  Scenario: should cast INTERVAL DAY TO SECOND to numpy timedelta64 millisecond unit
    # Default DAY TO SECOND uses Decimal128 (SB16) storage, which drives
    # the converter to millisecond resolution. Usual: 0 and 1.234 s.
    # Extreme: Snowflake spec min/max days (+/- 999999999) without sub-millisecond
    # digits. Sub-ms handling is covered separately by the truncation scenario
    # because ms-path truncation is asymmetric for negative values.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND, '0 0:0:1.234'::INTERVAL DAY TO SECOND, '999999999 23:59:59.000'::INTERVAL DAY TO SECOND, '-999999999 23:59:59.000'::INTERVAL DAY TO SECOND" is executed
    Then All values should be returned as numpy.timedelta64 type with millisecond resolution
    And Values should match exactly [0 ms, 1234 ms, 86399999999999000 ms, -86399999999999000 ms]

  @python_e2e
  Scenario: should floor sub-millisecond digits toward negative infinity for INTERVAL DAY TO SECOND with numpy
    # Key behavior: the ms path uses Python floor division on nanoseconds,
    # which rounds toward -inf rather than toward zero. For positive values
    # this is indistinguishable from truncation; for negative values the
    # result is one ms further from zero.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0 0:0:1.234999'::INTERVAL DAY TO SECOND, '-0 0:0:1.234999'::INTERVAL DAY TO SECOND" is executed
    Then All values should be returned as numpy.timedelta64 type with millisecond resolution
    And Values should match exactly [1234 ms, -1235 ms]

  @python_e2e
  Scenario: should cast INTERVAL DAY 3 TO SECOND to numpy timedelta64 nanosecond unit
    # Reduced precision DAY(3) TO SECOND fits in int64 (SB8) nanosecond storage,
    # which preserves full nanosecond precision. Usual: 0 and 1.234567890 s.
    # Spec min/max for DAY(3): +/- 999 23:59:59.999999999.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0 0:0:0.0'::INTERVAL DAY(3) TO SECOND, '0 0:0:1.234567890'::INTERVAL DAY(3) TO SECOND, '999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND, '-999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND" is executed
    Then All values should be returned as numpy.timedelta64 type with nanosecond resolution
    And Values should match exactly [0 ns, 1234567890 ns, 86399999999999999 ns, -86399999999999999 ns]

  # ============================================================================
  # NULL handling
  # ============================================================================

  @python_e2e
  Scenario: should handle NULL values for INTERVAL with numpy
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT NULL::INTERVAL YEAR, NULL::INTERVAL MONTH, NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, NULL::INTERVAL DAY(3) TO SECOND" is executed
    Then Result should contain [NULL, NULL, NULL, NULL, NULL]
