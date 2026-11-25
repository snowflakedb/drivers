package net.snowflake.client.api.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Collections;
import java.util.EnumMap;
import java.util.Map;
import net.snowflake.client.internal.loader.StreamLoader;
import org.junit.jupiter.api.Test;

public class LoaderFactoryTest {

  @Test
  public void shouldCreateStreamLoader() {
    // Given properties without columns, so no metadata lookup is needed
    Map<LoaderProperty, Object> properties = baseProperties(Operation.INSERT);

    // When a loader is created
    Loader loader = LoaderFactory.createLoader(properties, null, null);

    // Then it is a StreamLoader
    assertInstanceOf(StreamLoader.class, loader);
  }

  @Test
  public void shouldRejectCompressionLevelOutsideDeflaterRange() {
    // Given a compression level above Deflater.BEST_COMPRESSION
    Map<LoaderProperty, Object> properties = baseProperties(Operation.INSERT);
    properties.put(LoaderProperty.compressLevel, 10L);

    // When a loader is created
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> LoaderFactory.createLoader(properties, null, null));

    // Then the compression level is rejected
    assertEquals("invalid compression level", error.getMessage());
  }

  @Test
  public void shouldRejectNonIntegralBatchRowSize() {
    // Given a batchRowSize that is neither a String, Long nor Integer
    Map<LoaderProperty, Object> properties = baseProperties(Operation.INSERT);
    properties.put(LoaderProperty.batchRowSize, 1.5d);

    // When a loader is created
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> LoaderFactory.createLoader(properties, null, null));

    // Then the property is rejected by name
    assertEquals("'batchRowSize' Must be a LONG value", error.getMessage());
  }

  @Test
  public void shouldRequireKeysWhenStartingUpdatingOperation() {
    // Given a DELETE loader without keys
    Map<LoaderProperty, Object> properties = baseProperties(Operation.DELETE);
    properties.put(LoaderProperty.keys, Collections.emptyList());
    Loader loader = LoaderFactory.createLoader(properties, null, null);

    // When the loader starts
    Loader.ConnectionError error = assertThrows(Loader.ConnectionError.class, loader::start);

    // Then keys are required
    assertEquals("Updating operations require keys", error.getMessage());
  }

  private static Map<LoaderProperty, Object> baseProperties(Operation operation) {
    Map<LoaderProperty, Object> properties = new EnumMap<>(LoaderProperty.class);
    properties.put(LoaderProperty.tableName, "T");
    properties.put(LoaderProperty.schemaName, "S");
    properties.put(LoaderProperty.databaseName, "D");
    properties.put(LoaderProperty.operation, operation);
    return properties;
  }
}
