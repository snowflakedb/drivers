package net.snowflake.jdbc.e2e.types;

import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;

/** GeoJSON coordinate assertions shared by geography and geometry e2e tests. */
interface WithGeoAssertions {

  default void assertGeoJson(String value, String expectedType, JsonNode expectedCoordinates) {
    JsonNode geo = parseJson(value);
    assertEquals(expectedType, geo.get("type").asText());
    assertJsonArraysEqual(expectedCoordinates, geo.get("coordinates"));
  }

  static void assertJsonArraysEqual(JsonNode expected, JsonNode actual) {
    assertEquals(expected.size(), actual.size(), "Coordinate array length mismatch");
    for (int i = 0; i < expected.size(); i++) {
      JsonNode expectedItem = expected.get(i);
      JsonNode actualItem = actual.get(i);
      if (expectedItem.isArray()) {
        assertTrue(actualItem.isArray(), "Expected a nested coordinate array");
        assertJsonArraysEqual(expectedItem, actualItem);
      } else if (expectedItem.isNumber()) {
        assertTrue(actualItem.isNumber(), "Expected a numeric coordinate");
        assertEquals(expectedItem.asDouble(), actualItem.asDouble(), 1e-9);
      } else {
        assertEquals(expectedItem, actualItem);
      }
    }
  }
}
