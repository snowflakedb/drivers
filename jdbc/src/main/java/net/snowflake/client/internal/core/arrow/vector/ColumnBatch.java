package net.snowflake.client.internal.core.arrow.vector;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Getter;
import net.snowflake.client.internal.core.arrow.vector.Column.Field;
import net.snowflake.client.internal.core.arrow.vector.Column.PhysicalKind;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.NativeArrowBatch;
import net.snowflake.client.internal.unicore.NativeArrowBatch.NativeArrowArray;

@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class ColumnBatch implements AutoCloseable {
  private static final SFLogger logger = SFLoggerFactory.getLogger(ColumnBatch.class);

  private final List<Field> fields;
  private final List<Column> columns;
  @Getter private int rowCount;

  public static ColumnBatch of(Column... columns) {
    List<Field> fields = Arrays.stream(columns).map(Column::getField).collect(Collectors.toList());
    return new ColumnBatch(fields, Arrays.asList(columns), 0);
  }

  public static List<Field> decodeSchema(byte[] encoded) {
    Cursor cursor = new Cursor(encoded);
    return cursor.readFields(cursor.readInt());
  }

  public static ColumnBatch empty(List<Field> fields) {
    return new ColumnBatch(fields, columnsFrom(fields), 0);
  }

  public static ColumnBatch view(List<Field> fields, NativeArrowBatch batch) {
    NativeArrowArray[] arrays = batch.getColumns();
    if (fields.size() != arrays.length) {
      throw new IllegalArgumentException(
          "Schema field count "
              + fields.size()
              + " does not match batch column count "
              + arrays.length);
    }
    List<Column> columns = columnsFrom(fields);
    for (int index = 0; index < columns.size(); index++) {
      columns.get(index).bind(arrays[index], batch.getRowCount());
    }
    logger.debug("view: rows={} columns={}", batch.getRowCount(), columns.size());
    return new ColumnBatch(fields, columns, batch.getRowCount());
  }

  public List<Field> fields() {
    return fields;
  }

  public Column column(int index) {
    return columns.get(index);
  }

  public Column column(String name) {
    return columns.stream()
        .filter(column -> column.getField().getName().equals(name))
        .findFirst()
        .orElse(null);
  }

  public void setRowCount(int rowCount) {
    this.rowCount = rowCount;
    for (Column column : columns) {
      column.setValueCountWithChildren(rowCount);
    }
  }

  @Override
  public void close() {
    logger.debug("close: rows={} columns={}", rowCount, columns.size());
    columns.forEach(Column::close);
  }

  private static List<Column> columnsFrom(List<Field> fields) {
    return fields.stream().map(Column::fromField).collect(Collectors.toList());
  }

  private static final class Cursor {
    private final ByteBuffer buffer;

    private Cursor(byte[] encoded) {
      this.buffer = ByteBuffer.wrap(encoded).order(ByteOrder.LITTLE_ENDIAN);
    }

    private List<Field> readFields(int count) {
      List<Field> fields = new ArrayList<Field>(count);
      for (int index = 0; index < count; index++) {
        fields.add(readField());
      }
      return fields;
    }

    private Field readField() {
      String name = readString();
      Map<String, String> metadata = readMetadata();
      boolean nullable = buffer.get() != 0;
      PhysicalKind physicalKind = PhysicalKind.fromTag(buffer.get() & 0xff);
      switch (physicalKind) {
        case DECIMAL128:
          return Column.field(
              name,
              physicalKind,
              nullable,
              metadata,
              Collections.emptyList(),
              readInt(),
              readInt(),
              0);
        case STRUCT:
          return Column.field(
              name, physicalKind, nullable, metadata, readFields(readInt()), 0, 0, 0);
        case FIXED_SIZE_LIST:
          {
            int listSize = readInt();
            return Column.field(
                name, physicalKind, nullable, metadata, readFields(1), 0, 0, listSize);
          }
        default:
          return Column.field(
              name, physicalKind, nullable, metadata, Collections.emptyList(), 0, 0, 0);
      }
    }

    private Map<String, String> readMetadata() {
      int count = readInt();
      Map<String, String> metadata = new LinkedHashMap<String, String>();
      for (int index = 0; index < count; index++) {
        metadata.put(readString(), readString());
      }
      return metadata;
    }

    private String readString() {
      int length = readInt();
      if (length < 0) {
        throw new IllegalArgumentException("Negative byte length " + length);
      }
      byte[] bytes = new byte[length];
      buffer.get(bytes);
      return new String(bytes, StandardCharsets.UTF_8);
    }

    private int readInt() {
      return buffer.getInt();
    }
  }
}
