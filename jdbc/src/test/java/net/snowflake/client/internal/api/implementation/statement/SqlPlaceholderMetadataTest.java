package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

public class SqlPlaceholderMetadataTest {
  @Test
  public void testAnalyzePositionalPlaceholders() {
    SqlPlaceholderMetadata metadata = SqlPlaceholderMetadata.analyze("SELECT ?, ?, ?");

    assertEquals(
        SqlPlaceholderMetadata.PlaceholderStyle.POSITIONAL,
        metadata.placeholderStyle(),
        "Expected positional placeholder style");
    assertEquals(3, metadata.positionalPlaceholderCount(), "Expected positional placeholder count");
    assertEquals(Arrays.asList(1, 2, 3), new ArrayList<>(metadata.referencedParameterIndexes()));
  }

  @Test
  public void testAnalyzeNumericPlaceholdersKeepsCastHandlingSimple() {
    SqlPlaceholderMetadata metadata = SqlPlaceholderMetadata.analyze("SELECT :2, :1::VARCHAR, :2");

    assertEquals(
        SqlPlaceholderMetadata.PlaceholderStyle.NUMERIC,
        metadata.placeholderStyle(),
        "Expected numeric placeholder style");
    assertEquals(Arrays.asList(1, 2), new ArrayList<>(metadata.referencedParameterIndexes()));
    assertTrue(metadata.referencesParameterIndex(1), "Expected parameter 1 to be referenced");
    assertTrue(metadata.referencesParameterIndex(2), "Expected parameter 2 to be referenced");
    assertFalse(
        metadata.referencesParameterIndex(3), "Did not expect parameter 3 to be referenced");
  }

  @Test
  public void testAnalyzeMixedPlaceholders() {
    SqlPlaceholderMetadata metadata = SqlPlaceholderMetadata.analyze("SELECT ?, :1");

    assertEquals(
        SqlPlaceholderMetadata.PlaceholderStyle.MIXED,
        metadata.placeholderStyle(),
        "Expected mixed placeholder style");
    assertTrue(metadata.hasMixedPlaceholderStyles(), "Expected mixed placeholder flag");
  }
}
