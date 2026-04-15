@odbc
Feature: ODBC string to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: should convert string literals to SQL_C_BINARY
    Given Snowflake client is logged in
    When Query selecting various string literals is executed
    Then ASCII string 'hello' should convert to raw bytes
    And empty string should return 0 bytes
    And mixed ASCII with special characters should convert correctly
    And NULL should return SQL_NULL_DATA

  @odbc_e2e
  Scenario: should convert UTF-8 string literals to SQL_C_BINARY
    Given Snowflake client is logged in
    When Query selecting UTF-8 string literals is executed
    Then Japanese '日本語' should convert to raw bytes
    And Russian 'Привет' should convert to raw bytes
    And Chinese '你好' should convert to raw bytes
    And emoji string 'émoji: 😀' should include multi-byte emoji
    And French 'café' should convert correctly
    And Spanish 'Ñoño' should convert correctly
    And musical symbol '𝄞' should convert correctly
