package net.snowflake.client.internal.api.implementation.statement;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.apache.arrow.memory.ArrowBuf;
import org.apache.arrow.memory.RootAllocator;
import org.json.JSONStringer;

final class PreparedStatementBindingSerializer {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(PreparedStatementBindingSerializer.class);

  static final class ParameterValue {
    private final String bindType;
    private final Object value;

    ParameterValue(String bindType, Object value) {
      this.bindType = bindType;
      this.value = value;
    }

    String bindType() {
      return bindType;
    }

    Object value() {
      return value;
    }
  }

  static final class SerializedBindings implements AutoCloseable {
    private final QueryBindings bindings;
    private final NativeBuffer buffer;

    SerializedBindings(QueryBindings bindings, NativeBuffer buffer) {
      this.bindings = bindings;
      this.buffer = buffer;
    }

    QueryBindings bindings() {
      return bindings;
    }

    @Override
    public void close() {
      if (buffer != null) {
        buffer.close();
      }
    }
  }

  private PreparedStatementBindingSerializer() {}

  static SerializedBindings serialize(ParameterValue[] parameterValues) throws SQLException {
    if (parameterValues.length == 0) {
      logger.debug("No parameter placeholders found, skipping bindings serialization.");
      return new SerializedBindings(null, null);
    }
    logger.debug("Serializing prepared bindings: placeholders={}", parameterValues.length);

    JSONStringer jsonStringer = new JSONStringer();
    jsonStringer.object();
    for (int i = 0; i < parameterValues.length; i++) {
      ParameterValue parameterValue = parameterValues[i];
      int parameterIndex = i + 1;
      if (parameterValue == null) {
        logger.warn(
            "Bindings serialization failed: missing parameter value for index {}", parameterIndex);
        throw new SQLException("Missing value for parameter index: " + parameterIndex);
      }

      jsonStringer.key(String.valueOf(parameterIndex)).object();
      jsonStringer.key("type").value(parameterValue.bindType());
      if (parameterValue.value() == null) {
        jsonStringer.key("value").value(null);
      } else {
        jsonStringer.key("value").value(String.valueOf(parameterValue.value()));
      }
      jsonStringer.endObject();
    }
    jsonStringer.endObject();

    byte[] jsonBytes = jsonStringer.toString().getBytes(StandardCharsets.UTF_8);
    NativeBuffer nativeBuffer = NativeBuffer.fromBytes(jsonBytes);
    boolean success = false;
    try {
      byte[] ptrBytes = nativeBuffer.pointerAsLittleEndianBytes();
      BinaryDataPtr jsonPtr =
          BinaryDataPtr.newBuilder()
              .setValue(ByteString.copyFrom(ptrBytes))
              .setLength(jsonBytes.length)
              .build();
      QueryBindings queryBindings = QueryBindings.newBuilder().setJson(jsonPtr).build();
      logger.debug(
          "Prepared bindings serialized: payloadBytes={}, pointerBytes={}",
          jsonBytes.length,
          ptrBytes.length);
      SerializedBindings serializedBindings = new SerializedBindings(queryBindings, nativeBuffer);
      success = true;
      return serializedBindings;
    } finally {
      if (!success) {
        nativeBuffer.close();
      }
    }
  }

  private static final class NativeBuffer implements AutoCloseable {
    private final RootAllocator allocator;
    private final ArrowBuf arrowBuf;
    private final long address;

    private NativeBuffer(RootAllocator allocator, ArrowBuf arrowBuf, long address) {
      this.allocator = allocator;
      this.arrowBuf = arrowBuf;
      this.address = address;
    }

    private static NativeBuffer fromBytes(byte[] source) throws SQLException {
      RootAllocator allocator = null;
      ArrowBuf arrowBuf = null;
      boolean success = false;
      try {
        allocator = new RootAllocator(Long.MAX_VALUE);
        arrowBuf = allocator.buffer(source.length);
        arrowBuf.setBytes(0, source);
        long address = arrowBuf.memoryAddress();
        if (address == 0L) {
          logger.warn(
              "Failed to allocate native memory for binding data: payloadBytes={}", source.length);
          throw new SQLException("Failed to allocate native memory for binding data");
        }
        logger.debug("Allocated native binding buffer: payloadBytes={}", source.length);
        success = true;
        return new NativeBuffer(allocator, arrowBuf, address);
      } finally {
        if (!success) {
          if (arrowBuf != null) {
            arrowBuf.close();
          }
          if (allocator != null) {
            allocator.close();
          }
        }
      }
    }

    byte[] pointerAsLittleEndianBytes() {
      return ByteBuffer.allocate(Long.BYTES)
          .order(ByteOrder.LITTLE_ENDIAN)
          .putLong(address)
          .array();
    }

    @Override
    public void close() {
      arrowBuf.close();
      allocator.close();
    }
  }
}
