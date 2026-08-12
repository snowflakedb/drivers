package net.snowflake.client.internal.api.implementation.statement;

import com.fasterxml.jackson.core.JsonEncoding;
import com.fasterxml.jackson.core.JsonFactory;
import com.fasterxml.jackson.core.JsonGenerator;
import com.google.protobuf.ByteString;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.BinaryDataPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;
import org.apache.arrow.memory.ArrowBuf;
import org.apache.arrow.memory.RootAllocator;

final class PreparedStatementBindingSerializer {
  private static final SFLogger logger =
      SFLoggerFactory.getLogger(PreparedStatementBindingSerializer.class);

  /** Thread-safe once configured; reused to avoid re-parsing the (empty) generator config. */
  private static final JsonFactory JSON_FACTORY = new JsonFactory();

  /** Process-wide; a fresh allocator per execute is measurably expensive in batch scenarios. */
  static final RootAllocator SHARED_ALLOCATOR = new RootAllocator(Long.MAX_VALUE);

  static final class ParameterValue {
    private final SnowflakeType bindType;
    private final Object value;

    ParameterValue(SnowflakeType bindType, Object value) {
      this.bindType = bindType;
      this.value = value;
    }

    SnowflakeType bindType() {
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

  static NativeBindings serialize(Map<Integer, ParameterValue> parameterValues) {
    if (parameterValues.isEmpty()) {
      logger.debug("No parameter values bound, skipping bindings serialization.");
      return new NativeBindings(null, null);
    }
    logger.debug("Serializing prepared bindings: binds={}", parameterValues.size());

    byte[] jsonBytes = buildBindingsJson(parameterValues);
    return allocateNativeBindings(jsonBytes);
  }

  private static byte[] buildBindingsJson(Map<Integer, ParameterValue> parameterValues) {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    try (JsonGenerator json = JSON_FACTORY.createGenerator(out, JsonEncoding.UTF8)) {
      json.writeStartObject();
      // Emit every bound value, keyed by parameter index and ordered for a deterministic payload.
      // The server validates placeholder count/style/types — the driver never inspects the SQL.
      for (Map.Entry<Integer, ParameterValue> entry : new TreeMap<>(parameterValues).entrySet()) {
        int parameterIndex = entry.getKey();
        ParameterValue parameterValue = entry.getValue();
        json.writeObjectFieldStart(String.valueOf(parameterIndex));
        json.writeStringField("type", parameterValue.bindType().name());
        json.writeFieldName("value");
        writeBindingValue(json, parameterIndex, parameterValue.value());
        json.writeEndObject();
      }
      json.writeEndObject();
    } catch (IOException e) {
      // ByteArrayOutputStream never throws; this only fires on a genuine serialization fault.
      throw new SFSQLException("Failed to serialize prepared statement bindings", e);
    }
    return out.toByteArray();
  }

  private static void writeBindingValue(JsonGenerator json, int parameterIndex, Object value)
      throws IOException {
    if (value == null) {
      json.writeNull();
      return;
    }
    if (value instanceof String) {
      json.writeString((String) value);
      return;
    }
    if (value instanceof List) {
      json.writeStartArray();
      for (Object element : (List<?>) value) {
        requireNullOrString(element, parameterIndex, "list-valued binding");
        if (element == null) {
          json.writeNull();
        } else {
          json.writeString((String) element);
        }
      }
      json.writeEndArray();
      return;
    }
    throw unsupportedBindingValue(parameterIndex, value, "binding");
  }

  private static void requireNullOrString(Object element, int parameterIndex, String context) {
    if (element != null && !(element instanceof String)) {
      throw unsupportedBindingValue(parameterIndex, element, context);
    }
  }

  private static SFSQLException unsupportedBindingValue(
      int parameterIndex, Object value, String context) {
    return new SFSQLException(
        "Internal error: "
            + context
            + " for parameter "
            + parameterIndex
            + " has an unsupported value type ("
            + value.getClass().getCanonicalName()
            + ")");
  }

  private static NativeBindings allocateNativeBindings(byte[] jsonBytes) {
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

    private static NativeBuffer fromBytes(byte[] source) {
      ArrowBuf arrowBuf = null;
      boolean success = false;
      try {
        arrowBuf = SHARED_ALLOCATOR.buffer(source.length);
        arrowBuf.setBytes(0, source);
        long address = arrowBuf.memoryAddress();
        if (address == 0L) {
          logger.warn(
              "Failed to allocate native memory for binding data: payloadBytes={}", source.length);
          throw new SFSQLException("Failed to allocate native memory for binding data");
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
