package net.snowflake.client.internal.core.arrow.vector;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.math.RoundingMode;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.core.arrow.vector.Column.Field;
import net.snowflake.client.internal.core.arrow.vector.Column.PhysicalKind;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class ColumnBuilder {
  public static Column of(String name, PhysicalKind kind, Map<String, String> metadata) {
    return column(name, kind, metadata, Collections.<Column>emptyList(), 0, 0, 0);
  }

  public static Column decimal(
      String name, int precision, int scale, Map<String, String> metadata) {
    return column(
        name,
        PhysicalKind.DECIMAL128,
        metadata,
        Collections.<Column>emptyList(),
        precision,
        scale,
        0);
  }

  public static Column struct(String name, Map<String, String> metadata, Column... children) {
    return column(name, PhysicalKind.STRUCT, metadata, Arrays.asList(children), 0, 0, 0);
  }

  public static Column list(String name, Column child, int listSize) {
    return list(name, null, child, listSize);
  }

  public static Column list(String name, Map<String, String> metadata, Column child, int listSize) {
    return column(
        name,
        PhysicalKind.FIXED_SIZE_LIST,
        metadata,
        Collections.singletonList(child),
        0,
        0,
        listSize);
  }

  public static void set(Column column, int index, long value) {
    writeValidity(column, index, true);
    switch (column.physicalKind()) {
      case BOOL:
        writeBit(values(column, (index >> 3) + 1), index, (value & 1) != 0);
        break;
      case INT8:
        values(column, index + 1).put(index, (byte) value);
        break;
      case INT16:
        values(column, (index + 1) * Short.BYTES).putShort(index * Short.BYTES, (short) value);
        break;
      case INT32:
      case DATE32:
        values(column, (index + 1) * Integer.BYTES).putInt(index * Integer.BYTES, (int) value);
        break;
      case INT64:
        values(column, (index + 1) * Long.BYTES).putLong(index * Long.BYTES, value);
        break;
      default:
        throw new IllegalStateException(
            "Not a fixed-width integer column: " + column.physicalKind());
    }
  }

  public static void set(Column column, int index, float value) {
    if (column.physicalKind() == PhysicalKind.FLOAT64) {
      set(column, index, (double) value);
      return;
    }
    writeValidity(column, index, true);
    values(column, (index + 1) * Float.BYTES).putFloat(index * Float.BYTES, value);
  }

  public static void set(Column column, int index, double value) {
    writeValidity(column, index, true);
    values(column, (index + 1) * Double.BYTES).putDouble(index * Double.BYTES, value);
  }

  public static void set(Column column, int index, byte[] value) {
    int start = appendSlot(column, index, value.length);
    writeValidity(column, index, true);
    ByteBuffer values = values(column, start + value.length);
    System.arraycopy(value, 0, values.array(), values.arrayOffset() + start, value.length);
  }

  public static void set(Column column, int index, BigDecimal value) {
    writeValidity(column, index, true);
    BigInteger unscaled =
        value.setScale(column.getField().getScale(), RoundingMode.UNNECESSARY).unscaledValue();
    int width = 16;
    values(column, (index + 1) * width)
        .putLong(index * width, unscaled.longValue())
        .putLong(index * width + Long.BYTES, unscaled.shiftRight(64).longValue());
  }

  public static void setNull(Column column, int index) {
    if (column.physicalKind().isVariableWidth()) {
      appendSlot(column, index, 0);
    }
    writeValidity(column, index, false);
  }

  public static void setDefined(Column column, int index) {
    writeValidity(column, index, true);
  }

  public static void setValueCount(Column column, int count) {
    if (!column.nativeBound && count > 0) {
      ensureMutableValidity(column, count - 1);
    }
    column.setValueCountWithChildren(count);
  }

  public static void clear(Column column) {
    column.clear();
  }

  private static Column column(
      String name,
      PhysicalKind kind,
      Map<String, String> metadata,
      List<Column> children,
      int precision,
      int scale,
      int listSize) {
    List<Field> childFields = children.stream().map(Column::getField).collect(Collectors.toList());
    return new Column(
        Column.field(name, kind, true, metadata, childFields, precision, scale, listSize),
        children);
  }

  private static void writeValidity(Column column, int index, boolean valid) {
    ensureMutableValidity(column, index);
    writeBit(column.validity, index, valid);
    column.valueCount = Math.max(column.valueCount, index + 1);
  }

  private static void writeBit(ByteBuffer bits, int index, boolean set) {
    int byteIndex = index >> 3;
    int mask = 1 << (index & 7);
    byte current = bits.get(byteIndex);
    bits.put(byteIndex, (byte) (set ? current | mask : current & ~mask));
  }

  private static void ensureMutableValidity(Column column, int index) {
    column.checkOpen();
    ByteBuffer validity = column.validity;
    column.validity = grow(validity == null ? Column.emptyBuffer() : validity, (index >> 3) + 1);
  }

  private static ByteBuffer values(Column column, int bytes) {
    column.values = grow(column.values, bytes);
    return column.values;
  }

  private static int appendSlot(Column column, int index, int length) {
    ByteBuffer offsets = grow(column.offsets, (index + 2) * Integer.BYTES);
    column.offsets = offsets;
    for (int slot = column.filledThrough; slot < index; slot++) {
      offsets.putInt((slot + 1) * Integer.BYTES, offsets.getInt(slot * Integer.BYTES));
    }
    int start = offsets.getInt(index * Integer.BYTES);
    offsets.putInt((index + 1) * Integer.BYTES, start + length);
    column.filledThrough = index + 1;
    return start;
  }

  private static ByteBuffer grow(ByteBuffer buffer, int bytes) {
    if (buffer.capacity() >= bytes && buffer.hasArray()) {
      return buffer;
    }
    ByteBuffer grown = Column.emptyBuffer(Math.max(bytes, buffer.capacity() * 2 + 16));
    ByteBuffer view = buffer.duplicate();
    view.clear();
    grown.put(view);
    grown.clear();
    return grown;
  }
}
