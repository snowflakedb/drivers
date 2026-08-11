package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import org.json.JSONArray;
import org.json.JSONObject;

/** GeoJSON coordinate assertions shared by geography and geometry e2e tests. */
interface WithGeoAssertions {

  default void assertGeoJson(String value, String expectedType, JSONArray expectedCoordinates) {
    JSONObject geo = new JSONObject(value);
    assertEquals(expectedType, geo.getString("type"));
    assertJsonArraysEqual(expectedCoordinates, geo.getJSONArray("coordinates"));
  }

  static void assertJsonArraysEqual(JSONArray expected, JSONArray actual) {
    assertEquals(expected.length(), actual.length(), "Coordinate array length mismatch");
    for (int i = 0; i < expected.length(); i++) {
      Object expectedItem = expected.get(i);
      Object actualItem = actual.get(i);
      if (expectedItem instanceof JSONArray) {
        assertInstanceOf(JSONArray.class, actualItem);
        assertJsonArraysEqual((JSONArray) expectedItem, (JSONArray) actualItem);
      } else if (expectedItem instanceof Number) {
        assertInstanceOf(Number.class, actualItem);
        assertEquals(
            ((Number) expectedItem).doubleValue(), ((Number) actualItem).doubleValue(), 1e-9);
      } else {
        assertEquals(expectedItem, actualItem);
      }
    }
  }
}
