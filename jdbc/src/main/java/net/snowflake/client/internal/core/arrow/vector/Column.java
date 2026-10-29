package net.snowflake.client.internal.core.arrow.vector;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import lombok.Getter;
import lombok.RequiredArgsConstructor;
import lombok.Value;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

public final class Column implements AutoCloseable {
  /**
   * {@link #getTag()} matches the physical type tag written by {@code
   * jdbc_bridge/src/arrow_stream.rs}.
   */
  @Getter
  @RequiredArgsConstructor
  public enum PhysicalKind {
    BOOL(1),
    INT8(2),
    INT16(3),
    INT32(4),
    INT64(5),
    FLOAT32(6),
    FLOAT64(7),
    UTF8(8),
    BINARY(9),
    DATE32(10),
    DECIMAL128(11),
    STRUCT(12),
    FIXED_SIZE_LIST(13);

    private final int tag;

    static PhysicalKind fromTag(int tag) {
      for (PhysicalKind physicalKind : values()) {
        if (physicalKind.tag == tag) {
          return physicalKind;
        }
      }
      throw new IllegalArgumentException("Unsupported physical column tag " + tag);
    }

    boolean isVariableWidth() {
      return this == UTF8 || this == BINARY;
    }
  }

  @Value
  public static class Field {
    String name;
    PhysicalKind physicalKind;
    boolean nullable;
    Map<String, String> metadata;
    List<Field> children;
    int precision;
    int scale;
    int listSize;
  }

  /**
   * Buffer slots each Arrow node occupies in a {@link #bind} hand-off array. Matches {@code
   * BUFFERS_PER_NODE} in {@code jdbc_bridge/src/lib.rs}; change both together.
   */
  static final int BUFFERS_PER_NODE = 3;

  private static final int VALIDITY_SLOT = 0;
  private static final int OFFSETS_SLOT = 1;
  private static final int VALUES_SLOT = 2;

  @Getter private final Field field;
  private final List<Column> children;

  ByteBuffer validity;
  ByteBuffer values;
  ByteBuffer offsets;
  int valueCount;
  int filledThrough;
  boolean nativeBound;
  private boolean closed;

  Column(Field field, List<Column> children) {
    this.field = field;
    this.children = children;
    clear();
  }

  public PhysicalKind physicalKind() {
    return field.physicalKind;
  }

  public Column child(String name) {
    return children.stream().filter(c -> c.field.name.equals(name)).findFirst().orElse(null);
  }

  public boolean isNull(int index) {
    checkOpen();
    if (index < 0 || index >= valueCount) {
      return true;
    }
    return validity != null && ((validity.get(index >> 3) >> (index & 7)) & 1) == 0;
  }

  public byte getByte(int index) {
    checkOpen();
    return values.get(index);
  }

  public short getShort(int index) {
    checkOpen();
    return values.getShort(index * Short.BYTES);
  }

  public int getInt(int index) {
    checkOpen();
    return values.getInt(index * Integer.BYTES);
  }

  public long getLong(int index) {
    checkOpen();
    return values.getLong(index * Long.BYTES);
  }

  public float getFloat(int index) {
    checkOpen();
    return values.getFloat(index * Float.BYTES);
  }

  public double getDouble(int index) {
    checkOpen();
    return values.getDouble(index * Double.BYTES);
  }

  public int getBit(int index) {
    checkOpen();
    return (values.get(index >> 3) >> (index & 7)) & 1;
  }

  public long getSigned(int index) {
    switch (field.physicalKind) {
      case BOOL:
        return getBit(index);
      case INT8:
        return getByte(index);
      case INT16:
        return getShort(index);
      case INT32:
      case DATE32:
        return getInt(index);
      case INT64:
        return getLong(index);
      default:
        throw new IllegalStateException("Not an integer column: " + field.physicalKind);
    }
  }

  public byte[] getBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    int start = offsets.getInt(index * Integer.BYTES);
    int end = offsets.getInt((index + 1) * Integer.BYTES);
    byte[] value = new byte[end - start];
    ByteBuffer view = values.duplicate();
    view.position(start);
    view.get(value);
    return value;
  }

  public BigDecimal getDecimal(int index) {
    if (isNull(index)) {
      return null;
    }
    int width = 16;
    byte[] bigEndian = new byte[width];
    for (int offset = 0; offset < width; offset++) {
      bigEndian[width - 1 - offset] = values.get(index * width + offset);
    }
    return new BigDecimal(new BigInteger(bigEndian), field.scale);
  }

  public Object getObject(int index) {
    switch (field.physicalKind) {
      case STRUCT:
        return structValue(index);
      case FIXED_SIZE_LIST:
        return listValue(index);
      case DECIMAL128:
        return getDecimal(index);
      default:
        // TODO(SNOW-4248904): Compile-time exhaustiveness remains deferred until the Arrow Java
        // removal is complete.
        throw new IllegalStateException("Column " + field.name + " has no object value");
    }
  }

  /**
   * Binds this column and its children to {@code rows} rows of the batch described by {@code
   * slots}, starting at {@code slot}. Returns the first slot after this subtree.
   *
   * <p>{@code slots} is the flat hand-off {@code jdbc_bridge} fills: {@value #BUFFERS_PER_NODE}
   * entries per node in pre-order (validity, offsets, values), a parent before its children. Both
   * sides derive that walk from the same encoded schema, so the slot of every node is fixed for the
   * life of the result set and this column tree is rebound rather than rebuilt per batch.
   */
  int bind(ByteBuffer[] slots, int slot, int rows) {
    checkOpen();
    validity = readOnly(slots[slot + VALIDITY_SLOT]);
    ByteBuffer boundValues = readOnly(slots[slot + VALUES_SLOT]);
    values = boundValues == null ? emptyValues() : boundValues;
    if (field.physicalKind.isVariableWidth()) {
      ByteBuffer boundOffsets = readOnly(slots[slot + OFFSETS_SLOT]);
      offsets = boundOffsets == null ? emptyOffsets() : boundOffsets;
      filledThrough = rows;
    }
    valueCount = rows;
    nativeBound = true;
    int next = slot + BUFFERS_PER_NODE;
    for (Column child : children) {
      next = child.bind(slots, next, childRows(rows));
    }
    return next;
  }

  /** Slots this column and its children occupy in a {@link #bind} hand-off array. */
  int slotCount() {
    int count = BUFFERS_PER_NODE;
    for (Column child : children) {
      count += child.slotCount();
    }
    return count;
  }

  void setValueCountWithChildren(int count) {
    valueCount = count;
    for (Column child : children) {
      child.setValueCountWithChildren(childRows(count));
    }
  }

  private int childRows(int rows) {
    if (field.physicalKind != PhysicalKind.FIXED_SIZE_LIST) {
      return rows;
    }
    try {
      return Math.multiplyExact(rows, field.listSize);
    } catch (ArithmeticException overflow) {
      SFSQLException failure =
          new SFSQLException(
              ErrorCode.INTERNAL_ERROR, "Arrow list column exceeds the supported row count");
      failure.initCause(overflow);
      throw failure;
    }
  }

  @Override
  public void close() {
    if (closed) {
      return;
    }
    closed = true;
    clear();
    offsets = emptyValues();
    children.forEach(Column::close);
  }

  private Map<String, Object> structValue(int index) {
    if (isNull(index)) {
      return null;
    }
    Map<String, Object> value = new LinkedHashMap<String, Object>();
    for (Column child : children) {
      value.put(child.field.name, child.elementValue(index));
    }
    return value;
  }

  private List<?> listValue(int index) {
    if (isNull(index)) {
      return null;
    }
    List<Object> value = new CompactList<Object>(field.listSize);
    Column child = children.get(0);
    int start = index * field.listSize;
    for (int offset = 0; offset < field.listSize; offset++) {
      value.add(child.elementValue(start + offset));
    }
    return value;
  }

  private Object elementValue(int index) {
    if (isNull(index)) {
      return null;
    }
    switch (field.physicalKind) {
      case BOOL:
        return getBit(index) != 0;
      case INT8:
        return getByte(index);
      case INT16:
        return getShort(index);
      case INT32:
      case DATE32:
        return getInt(index);
      case INT64:
        return getLong(index);
      case FLOAT32:
        return getFloat(index);
      case FLOAT64:
        return getDouble(index);
      case UTF8:
      case BINARY:
        return getBytes(index);
      case DECIMAL128:
        return getDecimal(index);
      default:
        throw new IllegalArgumentException("Unsupported struct child " + field.physicalKind);
    }
  }

  /**
   * Drops every reference to native batch memory and resets the counts. Unlike {@link #close()}
   * this column stays usable, so a reader can clear the tree before the native batch behind it is
   * freed and then rebind it to the next one.
   */
  void clear() {
    valueCount = 0;
    validity = null;
    nativeBound = false;
    values = emptyValues();
    filledThrough = 0;
    offsets = field.physicalKind.isVariableWidth() ? emptyOffsets() : emptyValues();
  }

  void clearWithChildren() {
    clear();
    for (Column child : children) {
      child.clearWithChildren();
    }
  }

  void checkOpen() {
    if (closed) {
      throw new IllegalStateException("Arrow vector is closed");
    }
  }

  static Column fromField(Field field) {
    return new Column(
        field, field.children.stream().map(Column::fromField).collect(Collectors.toList()));
  }

  static Field field(
      String name,
      PhysicalKind physicalKind,
      boolean nullable,
      Map<String, String> metadata,
      List<Field> children,
      int precision,
      int scale,
      int listSize) {
    Map<String, String> stored =
        metadata == null
            ? Collections.<String, String>emptyMap()
            : Collections.unmodifiableMap(new LinkedHashMap<String, String>(metadata));
    return new Field(
        name,
        physicalKind,
        nullable,
        stored,
        Collections.unmodifiableList(new ArrayList<Field>(children)),
        precision,
        scale,
        listSize);
  }

  /**
   * Returns a read-only little-endian view of a buffer handed over by {@code jdbc_bridge}, or
   * {@code null} when the role is absent. The batch memory belongs to the native stream, so Java
   * only ever reads it.
   */
  private static ByteBuffer readOnly(ByteBuffer buffer) {
    if (buffer == null) {
      return null;
    }
    ByteBuffer view = buffer.asReadOnlyBuffer();
    view.clear();
    // On Java 8 asReadOnlyBuffer() resets the byte order to big-endian, so this has to
    // run after the conversion, not before it. Arrow C data is little-endian.
    return view.order(ByteOrder.LITTLE_ENDIAN);
  }

  /**
   * Shared stand-ins for an absent values or offsets buffer. Every read through them is absolute
   * and bounds-checked, and they are read-only, so one instance is safe for every cleared column.
   * This keeps a rebound batch from allocating per column per batch.
   */
  private static final ByteBuffer EMPTY_VALUES = readOnly(emptyBuffer(0));

  private static final ByteBuffer EMPTY_OFFSETS = readOnly(emptyBuffer(Integer.BYTES));

  private static ByteBuffer emptyValues() {
    return EMPTY_VALUES;
  }

  private static ByteBuffer emptyOffsets() {
    return EMPTY_OFFSETS;
  }

  static ByteBuffer emptyBuffer() {
    return emptyBuffer(0);
  }

  static ByteBuffer emptyBuffer(int capacity) {
    return ByteBuffer.allocate(capacity).order(ByteOrder.LITTLE_ENDIAN);
  }

  private static final class CompactList<E> extends ArrayList<E> {
    private CompactList(int initialCapacity) {
      super(initialCapacity);
    }

    @Override
    public String toString() {
      return stream().map(String::valueOf).collect(Collectors.joining(",", "[", "]"));
    }
  }
}
