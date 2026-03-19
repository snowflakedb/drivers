@core_not_needed
Feature: String representation of binary data 
  # BINARY_OUTPUT_FORMAT controls how binary data is encoded when converted to STRING.
  # This affects explicit string conversion methods (e.g., getColumnValueAsString in Node.js)
  # if supported by the driver.
  # Formats:
  #   - HEX: Each byte encoded as 2 uppercase hex characters (e.g., 0x01 → "01")
  #   - BASE64: Standard Base64 encoding with padding (e.g., 0x01 0x23 0x45 → "ASNF")

  Scenario: should encode binary as HEX string when BINARY_OUTPUT_FORMAT is HEX
    # HEX encoding: each byte becomes 2 uppercase hexadecimal characters
    # Example: bytes 0x01 0x23 0x45 0x67 0x89 0xAB 0xCD 0xEF → "0123456789ABCDEF"
    Given Snowflake client is logged in
    And Session parameter BINARY_OUTPUT_FORMAT is set to 'HEX'
    When Query "SELECT X'0123456789ABCDEF' AS bin" is executed
    And bin is converted to string representation
    Then the string should be '0123456789ABCDEF'

  Scenario: should encode binary as BASE64 string when BINARY_OUTPUT_FORMAT is BASE64
    # BASE64 encoding: standard Base64 with padding
    # Example: bytes 0x01 0x23 0x45 0x67 0x89 0xAB 0xCD 0xEF → "ASNFZ4mrze8="
    Given Snowflake client is logged in
    And Session parameter BINARY_OUTPUT_FORMAT is set to 'BASE64'
    When Query "SELECT X'0123456789ABCDEF' AS bin" is executed
    And bin is converted to string representation
    Then the string should be 'ASNFZ4mrze8='
