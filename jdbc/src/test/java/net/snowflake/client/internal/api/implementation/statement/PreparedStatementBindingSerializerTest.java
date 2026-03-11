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
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.junit.jupiter.api.Test;

public class PreparedStatementBindingSerializerTest {

  @Test
  public void testSerializeEmptyParametersReturnsNullBindings() throws Exception {
    PreparedStatementBindingSerializer.ParameterValue[] params =
        new PreparedStatementBindingSerializer.ParameterValue[0];

    try (PreparedStatementBindingSerializer.SerializedBindings serialized =
        PreparedStatementBindingSerializer.serialize(params)) {
      assertNull(serialized.bindings(), "Expected null bindings for empty parameter list");
    }
  }

  @Test
  public void testSerializeMissingParameterFailsWithIndex() {
    PreparedStatementBindingSerializer.ParameterValue[] params =
        new PreparedStatementBindingSerializer.ParameterValue[2];
    params[0] = new PreparedStatementBindingSerializer.ParameterValue("TEXT", "hello");

    SQLException ex =
        assertThrows(
            SQLException.class, () -> PreparedStatementBindingSerializer.serialize(params));
    assertTrue(
        ex.getMessage().contains("Missing value for parameter index: 2"),
        "Expected missing-parameter index in error message");
  }

  @Test
  public void testSerializeCreatesJsonBindingsWithExpectedPointerMetadata() throws Exception {
    PreparedStatementBindingSerializer.ParameterValue[] params =
        new PreparedStatementBindingSerializer.ParameterValue[] {
          new PreparedStatementBindingSerializer.ParameterValue("FIXED", "42"),
          new PreparedStatementBindingSerializer.ParameterValue("TEXT", "hello")
        };

    String expectedJson =
        "{\"1\":{\"type\":\"FIXED\",\"value\":\"42\"},\"2\":{\"type\":\"TEXT\",\"value\":\"hello\"}}";
    byte[] expectedJsonBytes = expectedJson.getBytes(StandardCharsets.UTF_8);

    try (PreparedStatementBindingSerializer.SerializedBindings serialized =
        PreparedStatementBindingSerializer.serialize(params)) {
      QueryBindings bindings = serialized.bindings();
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
}
