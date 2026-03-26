package net.snowflake.client.internal.api.implementation.statement;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.Map;
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

  /** Holds bindings plus the native buffer backing the pointer stored in the RPC payload. */
  static final class NativeBindings implements AutoCloseable {
    private final QueryBindings bindings;
    private final NativeBuffer buffer;

    NativeBindings(QueryBindings bindings, NativeBuffer buffer) {
      this.bindings = bindings;
      this.buffer = buffer;
    }

    QueryBindings bindings() {
      return bindings;
    }

    @Override
    public void close() {
      // The bindings payload includes a pointer into this native buffer, so the owner must release
      // it after the RPC has been constructed and sent.
      if (buffer != null) {
        buffer.close();
      }
    }
  }

  private PreparedStatementBindingSerializer() {}

  static NativeBindings serialize(
      SqlPlaceholderMetadata placeholderMetadata, Map<Integer, ParameterValue> parameterValues)
      throws SQLException {
    if (!placeholderMetadata.hasBindings()) {
      logger.debug("No parameter placeholders found, skipping bindings serialization.");
      return new NativeBindings(null, null);
    }
    logger.debug(
        "Serializing prepared bindings: placeholders={}", placeholderMetadata.placeholderCount());

    byte[] jsonBytes = buildBindingsJson(placeholderMetadata, parameterValues);
    return allocateNativeBindings(jsonBytes);
  }

  private static byte[] buildBindingsJson(
      SqlPlaceholderMetadata placeholderMetadata, Map<Integer, ParameterValue> parameterValues)
      throws SQLException {
    JSONStringer jsonStringer = new JSONStringer();
    jsonStringer.object();
    for (int parameterIndex : placeholderMetadata.referencedParameterIndexes()) {
      ParameterValue parameterValue = parameterValues.get(parameterIndex);
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
    return jsonStringer.toString().getBytes(StandardCharsets.UTF_8);
  }

  private static NativeBindings allocateNativeBindings(byte[] jsonBytes) throws SQLException {
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
      NativeBindings nativeBindings = new NativeBindings(queryBindings, nativeBuffer);
      success = true;
      return nativeBindings;
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
