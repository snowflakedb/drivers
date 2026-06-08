package net.snowflake.client.internal.api.implementation.statement;

import com.google.protobuf.ByteString;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.List;
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

  /** Process-wide; a fresh allocator per execute is measurably expensive in batch scenarios. */
  static final RootAllocator SHARED_ALLOCATOR = new RootAllocator(Long.MAX_VALUE);

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

  /**
   * The {@link BinaryDataPtr} address is a raw {@code long}, not a Java reference to the underlying
   * {@link ArrowBuf}. Callers must keep this object reachable until the synchronous {@code
   * statementExecuteQuery} returns — the standard try-with-resources around the RPC suffices per
   * JLS §12.6.1.
   */
  static final class NativeBindings implements AutoCloseable {
    private final QueryBindings bindings;
    private final NativeBuffer buffer;
    private boolean closed;

    NativeBindings(QueryBindings bindings, NativeBuffer buffer) {
      this.bindings = bindings;
      this.buffer = buffer;
    }

    QueryBindings bindings() {
      return bindings;
    }

    @Override
    public void close() {
      // Idempotent — guards against double-close in nested try-with-resources / finally chains.
      if (closed) {
        return;
      }
      closed = true;
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
      jsonStringer.key("value");
      writeBindingValue(jsonStringer, parameterIndex, parameterValue.value());
      jsonStringer.endObject();
    }
    jsonStringer.endObject();
    return jsonStringer.toString().getBytes(StandardCharsets.UTF_8);
  }

  private static void writeBindingValue(JSONStringer json, int parameterIndex, Object value)
      throws SQLException {
    if (value == null || value instanceof String) {
      json.value(value);
      return;
    }
    if (value instanceof List) {
      json.array();
      for (Object element : (List<?>) value) {
        requireNullOrString(element, parameterIndex, "list-valued binding");
        json.value(element);
      }
      json.endArray();
      return;
    }
    throw unsupportedBindingValue(parameterIndex, value, "binding");
  }

  private static void requireNullOrString(Object element, int parameterIndex, String context)
      throws SQLException {
    if (element != null && !(element instanceof String)) {
      throw unsupportedBindingValue(parameterIndex, element, context);
    }
  }

  private static SQLException unsupportedBindingValue(
      int parameterIndex, Object value, String context) {
    return new SQLException(
        "Internal error: "
            + context
            + " for parameter "
            + parameterIndex
            + " has an unsupported value type ("
            + value.getClass().getCanonicalName()
            + ")");
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
    private final ArrowBuf arrowBuf;
    private final long address;
    private boolean closed;

    private NativeBuffer(ArrowBuf arrowBuf, long address) {
      this.arrowBuf = arrowBuf;
      this.address = address;
    }

    private static NativeBuffer fromBytes(byte[] source) throws SQLException {
      ArrowBuf arrowBuf = null;
      boolean success = false;
      try {
        arrowBuf = SHARED_ALLOCATOR.buffer(source.length);
        arrowBuf.setBytes(0, source);
        long address = arrowBuf.memoryAddress();
        if (address == 0L) {
          logger.warn(
              "Failed to allocate native memory for binding data: payloadBytes={}", source.length);
          throw new SQLException("Failed to allocate native memory for binding data");
        }
        logger.debug("Allocated native binding buffer: payloadBytes={}", source.length);
        success = true;
        return new NativeBuffer(arrowBuf, address);
      } finally {
        if (!success && arrowBuf != null) {
          arrowBuf.close();
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
      if (closed) {
        return;
      }
      closed = true;
      arrowBuf.close();
    }
  }
}
