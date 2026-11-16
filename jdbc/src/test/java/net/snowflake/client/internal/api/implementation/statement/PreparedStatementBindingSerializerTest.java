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
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.unicore.NativeBuffer;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

public class PreparedStatementBindingSerializerTest {

  private static long nativeBufferBaseline;

  @BeforeAll
  public static void snapshotNativeBufferBaseline() {
    nativeBufferBaseline = NativeBuffer.allocatedBytes();
  }

  // Each test closes its NativeBindings through try-with-resources; a serializer path that opened a
  // NativeBuffer and forgot to release it would raise the process-wide counter above the baseline
  // captured here. Reads a delta rather than zero because other test classes in the same JVM may
  // have left their own baseline before this class started.
  @AfterAll
  public static void assertNoNativeBufferLeak() {
    assertEquals(
        nativeBufferBaseline,
        NativeBuffer.allocatedBytes(),
        "Native binding buffers leaked after prepared-statement binding serializer tests");
  }

  @Test
  public void shouldReturnNullBindingsWhenNoParametersBound() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(params)) {
      assertNull(nativeBindings.bindings(), "Expected null bindings for empty parameter list");
    }
  }

  @Test
  public void shouldSerializeBoundValuesToJsonWithExpectedPointerMetadata() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(1, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.FIXED, "42"));
    params.put(
        2, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.TEXT, "hello"));

    String expectedJson =
        "{\"1\":{\"type\":\"FIXED\",\"value\":\"42\"},\"2\":{\"type\":\"TEXT\",\"value\":\"hello\"}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(params)) {
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
  public void shouldSerializeNonContiguousIndexesSortedByIndex() throws Exception {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> params = new HashMap<>();
    params.put(4, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.FIXED, "4"));
    params.put(2, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.TEXT, "two"));

    String expectedJson =
        "{\"2\":{\"type\":\"TEXT\",\"value\":\"two\"},\"4\":{\"type\":\"FIXED\",\"value\":\"4\"}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.NativeBindings nativeBindings =
        PreparedStatementBindingSerializer.serialize(params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings");
      assertEquals(
          expectedJsonBytes.length,
          bindings.getJson().getLength(),
          "JSON byte length should match sparse-index payload length");
    }
  }

  @Test
  public void shouldEmitJsonArrayWithNullSlotsForListValuedParameter() throws Exception {
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
        PreparedStatementBindingSerializer.serialize(params)) {
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
        PreparedStatementBindingSerializer.serialize(params)) {
      QueryBindings bindings = nativeBindings.bindings();
      assertNotNull(bindings, "Expected non-null bindings for timestamp array bind");
      assertTrue(bindings.hasJson(), "Expected JSON variant for timestamp array bind");
      assertEquals(
          expectedJsonBytes.length,
          bindings.getJson().getLength(),
          "Timestamp array-bind JSON byte length should match the verbatim epoch-nanos payload");
    }
  }

  @Test
  public void shouldRejectNonStringScalarAndListElementBindingValues() {
    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> badScalar = new HashMap<>();
    badScalar.put(
        1, new PreparedStatementBindingSerializer.ParameterValue(SnowflakeType.FIXED, 123));

    SFSQLException scalarEx =
        assertThrows(
            SFSQLException.class, () -> PreparedStatementBindingSerializer.serialize(badScalar));
    assertEquals(
        "Internal error: binding for parameter 1 has an unsupported value type (java.lang.Integer)",
        scalarEx.getMessage());

    Map<Integer, PreparedStatementBindingSerializer.ParameterValue> badListElement =
        new HashMap<>();
    badListElement.put(
        2,
        new PreparedStatementBindingSerializer.ParameterValue(
            SnowflakeType.FIXED, Arrays.asList("ok", 456)));

    SFSQLException listEx =
        assertThrows(
            SFSQLException.class,
            () -> PreparedStatementBindingSerializer.serialize(badListElement));
    assertEquals(
        "Internal error: list-valued binding for parameter 2 has an unsupported value type"
            + " (java.lang.Integer)",
        listEx.getMessage());
  }
}
