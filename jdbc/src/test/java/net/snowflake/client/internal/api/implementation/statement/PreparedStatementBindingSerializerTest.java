package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Test;

public class PreparedStatementBindingSerializerTest {

  /**
   * Detect ArrowBuf leaks. Any code path inside {@code serialize()} that allocates a buffer but
   * fails to close it (e.g. a future refactor that throws between {@code allocator.buffer(…)} and
   * {@code NativeBindings} construction) leaves bytes accounted on the shared allocator; this
   * assertion fires immediately after the test class completes.
   */
  @AfterAll
  public static void assertSharedAllocatorEmpty() {
    assertEquals(
        0L,
        PreparedStatementBindingSerializer.SHARED_ALLOCATOR.getAllocatedMemory(),
        "ArrowBuf leak: shared allocator still has bytes after binding serializer tests");
  }

  @Test
  public void testSerializeEmptyParametersReturnsNullBindings() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(
            SqlPlaceholderMetadata.analyze("SELECT 1"), params)) {
      assertNull(nativeBindings.bindings(), "Expected null bindings for empty parameter list");
    }
  }

  @Test
  public void testSerializeMissingParameterFailsWithIndex() {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(
        1, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.TEXT, "hello"));

    SQLException ex =
        assertThrows(
            SQLException.class,
            () ->
                PreparedStatementBindingSerializer.serialize(
                    SqlPlaceholderMetadata.analyze("SELECT ?, ?"), params));
    assertTrue(
        ex.getMessage().contains("Missing value for parameter index: 2"),
        "Expected missing-parameter index in error message");
  }

  @Test
  public void testSerializeCreatesJsonBindingsWithExpectedPointerMetadata() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(1, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.FIXED, "42"));
    params.put(
        2, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.TEXT, "hello"));

    String expectedJson =
        "{\"1\":{\"type\":\"FIXED\",\"value\":\"42\"},\"2\":{\"type\":\"TEXT\",\"value\":\"hello\"}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(
            SqlPlaceholderMetadata.analyze("SELECT ?, ?"), params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings");
      assertTrue(bindings.hasJson(), "Expected JSON query bindings");

      BinaryDataPtr jsonPtr = bindings.getJson();
      assertEquals(
          expectedJsonBytes.length,
          jsonPtr.getLength(),
          "JSON byte length should match serialized payload length");
      assertEquals(Long.BYTES, jsonPtr.getValue().size(), "Pointer payload should be 8 bytes");

      long pointerValue =
          ByteBuffer.wrap(jsonPtr.getValue().toByteArray())
              .order(ByteOrder.LITTLE_ENDIAN)
              .getLong();
      assertNotEquals(0L, pointerValue, "Native pointer value should not be zero");
    }
  }

  @Test
  public void testSerializeNumericPlaceholdersUsesReferencedIndexes() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(2, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.TEXT, "two"));
    params.put(4, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.FIXED, "4"));

    String expectedJson =
        "{\"2\":{\"type\":\"TEXT\",\"value\":\"two\"},\"4\":{\"type\":\"FIXED\",\"value\":\"4\"}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(
            SqlPlaceholderMetadata.analyze("SELECT :4, :2, :4"), params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings");
      assertEquals(
          expectedJsonBytes.length,
          bindings.getJson().getLength(),
          "JSON byte length should match numeric placeholder payload length");
    }
  }

  @Test
  public void testSerializeListValuedParameterEmitsJsonArrayWithNullSlots() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(
        1,
        new PreparedStatementBindingSerializer.ParameterValue(
            SnowflakeType.FIXED, Arrays.asList("1", "2", null, "4")));
    params.put(
        2,
        new PreparedStatementBindingSerializer.ParameterValue(
            SnowflakeType.TEXT, Arrays.asList("a", null, "c", "d")));

    String expectedJson =
        "{\"1\":{\"type\":\"FIXED\",\"value\":[\"1\",\"2\",null,\"4\"]},"
            + "\"2\":{\"type\":\"TEXT\",\"value\":[\"a\",null,\"c\",\"d\"]}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(
            SqlPlaceholderMetadata.analyze("INSERT INTO t VALUES (?, ?)"), params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings for array bind");
      assertTrue(bindings.hasJson(), "Expected JSON variant for array bind");
      assertEquals(
          expectedJsonBytes.length,
          bindings.getJson().getLength(),
          "Array-bind JSON byte length should match the canonical payload");
    }
  }

  @Test
  public void shouldPassTimestampBatchBindsThroughAsRawEpochNanosArraysWithoutReformat()
      throws Exception {
    // Batch timestamp binds go inline as JSON arrays of the raw wire strings the setters produced:
    // LTZ/NTZ are epoch-nanos decimal strings, TZ is "<nanos> <offsetCode>". There is no Java-side
    // stage/CSV reformat in this tree, so the serializer must emit these verbatim (a reformat to a
    // human-readable "yyyy-MM-dd ..." string would change the payload length below).
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(
        1,
        new PreparedStatementBindingSerializer.ParameterValue(
            SnowflakeType.TIMESTAMP_LTZ, Arrays.asList("1705323296789012345", "-1999999999")));
    params.put(
        2,
        new PreparedStatementBindingSerializer.ParameterValue(
            SnowflakeType.TIMESTAMP_TZ,
            Arrays.asList("1705323296789012345 1560", "1705323296789012345 960")));

    String expectedJson =
        "{\"1\":{\"type\":\"TIMESTAMP_LTZ\",\"value\":[\"1705323296789012345\",\"-1999999999\"]},"
            + "\"2\":{\"type\":\"TIMESTAMP_TZ\",\"value\":"
            + "[\"1705323296789012345 1560\",\"1705323296789012345 960\"]}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(
            SqlPlaceholderMetadata.analyze("INSERT INTO t VALUES (?, ?)"), params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings for timestamp array bind");
      assertTrue(bindings.hasJson(), "Expected JSON variant for timestamp array bind");
      assertEquals(
          expectedJsonBytes.length,
          bindings.getJson().getLength(),
          "Timestamp array-bind JSON byte length should match the verbatim epoch-nanos payload");
    }
  }
}
