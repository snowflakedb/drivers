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
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.junit.jupiter.api.Test;

public class PreparedStatementBindingSerializerTest {

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
    params.put(1, new PreparedStatementBindingSerializer.ParameterValue("TEXT", "hello"));

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
    params.put(1, new PreparedStatementBindingSerializer.ParameterValue("FIXED", "42"));
    params.put(2, new PreparedStatementBindingSerializer.ParameterValue("TEXT", "hello"));

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
    params.put(2, new PreparedStatementBindingSerializer.ParameterValue("TEXT", "two"));
    params.put(4, new PreparedStatementBindingSerializer.ParameterValue("FIXED", "4"));

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
}
