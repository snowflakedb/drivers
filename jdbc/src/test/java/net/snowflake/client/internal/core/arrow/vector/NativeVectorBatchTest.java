package net.snowflake.client.internal.core.arrow.vector;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.Column.PhysicalKind;
import net.snowflake.client.internal.unicore.NativeArrowBatch;
import net.snowflake.client.internal.unicore.NativeArrowBatch.NativeArrowArray;
import org.junit.jupiter.api.Test;

class NativeVectorBatchTest {

  @Test
  void shouldViewPhysicalIntAndStructColumns() {
    ByteBuffer numbers = direct(8).putLong(0, 42);
    ByteBuffer epoch = direct(8).putLong(0, 7);
    NativeArrowBatch batch =
        new NativeArrowBatch(1, new NativeArrowArray[] {array(numbers), array(null, array(epoch))});

    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(
                field("n", PhysicalKind.INT64, null),
                field("ts", PhysicalKind.STRUCT, field("epoch", PhysicalKind.INT64, null), 1)));

    ColumnBatch root = ColumnBatch.view(schema, batch);
    Column number = root.column("n");
    Column timestamp = root.column("ts");
    Column epochVector = timestamp.child("epoch");

    assertEquals(1, root.getRowCount());
    assertSame(number, root.column(0));
    assertSame(timestamp, root.column(1));
    assertFalse(number.isNull(0));
    assertEquals(42, number.getLong(0));
    assertFalse(timestamp.isNull(0));
    assertEquals(7, epochVector.getLong(0));
    assertTrue(number.getField().isNullable());
    assertEquals(Column.PhysicalKind.STRUCT, timestamp.physicalKind());
  }

  @Test
  void shouldReadAllPhysicalKindsAndPropagateRowCountAndClose() {
    Column boolCol = ColumnBuilder.of("b", PhysicalKind.BOOL, null);
    Column i8Col = ColumnBuilder.of("i8", PhysicalKind.INT8, null);
    Column i16Col = ColumnBuilder.of("i16", PhysicalKind.INT16, null);
    Column i32Col = ColumnBuilder.of("i32", PhysicalKind.INT32, null);
    Column dateCol = ColumnBuilder.of("d32", PhysicalKind.DATE32, null);
    Column i64Col = ColumnBuilder.of("i64", PhysicalKind.INT64, null);
    Column f32Col = ColumnBuilder.of("f32", PhysicalKind.FLOAT32, null);
    Column f64Col = ColumnBuilder.of("f64", PhysicalKind.FLOAT64, null);
    Column binCol = ColumnBuilder.of("bin", PhysicalKind.BINARY, null);
    Column decCol = ColumnBuilder.decimal("dec", 18, 2, null);
    Column listChild = ColumnBuilder.of("item", PhysicalKind.INT32, null);
    Column listCol = ColumnBuilder.list("vec", listChild, 2);

    // Index 8 crosses from byte 0 into byte 1 of the packed validity and value bitmaps.
    ColumnBuilder.set(boolCol, 0, 1L);
    ColumnBuilder.set(boolCol, 7, 0L);
    ColumnBuilder.set(boolCol, 8, 1L);
    ColumnBuilder.set(boolCol, 9, 0L);

    ColumnBuilder.set(i8Col, 0, -5L);
    ColumnBuilder.set(i16Col, 0, -1_234L);
    ColumnBuilder.set(i32Col, 0, -100_000L);
    ColumnBuilder.set(dateCol, 0, 19_737L);
    ColumnBuilder.set(i64Col, 0, -9_000_000_000L);
    ColumnBuilder.set(f32Col, 0, 1.5f);
    ColumnBuilder.set(f64Col, 0, -2.25d);
    ColumnBuilder.set(binCol, 0, new byte[] {0x01, (byte) 0xFF});
    ColumnBuilder.setNull(binCol, 1);
    ColumnBuilder.set(decCol, 0, new BigDecimal("-12345.67"));
    ColumnBuilder.setNull(decCol, 1);
    ColumnBuilder.set(listChild, 0, 10L);
    ColumnBuilder.set(listChild, 1, 20L);
    ColumnBuilder.set(listChild, 2, 30L);
    ColumnBuilder.set(listChild, 3, 40L);
    ColumnBuilder.setDefined(listCol, 0);
    ColumnBuilder.setDefined(listCol, 1);

    try (ColumnBatch batch =
        ColumnBatch.of(
            boolCol, i8Col, i16Col, i32Col, dateCol, i64Col, f32Col, f64Col, binCol, decCol,
            listCol)) {
      batch.setRowCount(2);
      assertEquals(2, batch.getRowCount());
      assertSame(boolCol, batch.column(0));
      assertSame(listCol, batch.column("vec"));
      assertNull(batch.column("missing"));

      assertEquals(1, boolCol.getBit(0));
      assertEquals(1L, boolCol.getSigned(0));
      assertEquals(0, boolCol.getBit(7));
      assertEquals(1, boolCol.getBit(8));
      assertEquals(0, boolCol.getBit(9));

      assertEquals((byte) -5, i8Col.getByte(0));
      assertEquals(-5L, i8Col.getSigned(0));
      assertEquals((short) -1_234, i16Col.getShort(0));
      assertEquals(-1_234L, i16Col.getSigned(0));
      assertEquals(-100_000, i32Col.getInt(0));
      assertEquals(-100_000L, i32Col.getSigned(0));
      assertEquals(19_737, dateCol.getInt(0));
      assertEquals(19_737L, dateCol.getSigned(0));
      assertEquals(-9_000_000_000L, i64Col.getLong(0));
      assertEquals(-9_000_000_000L, i64Col.getSigned(0));
      assertEquals(1.5f, f32Col.getFloat(0));
      assertEquals(-2.25d, f64Col.getDouble(0));

      assertEquals(2, binCol.getBytes(0).length);
      assertNull(binCol.getBytes(1));
      assertEquals(new BigDecimal("-12345.67"), decCol.getDecimal(0));
      assertEquals(new BigDecimal("-12345.67"), decCol.getObject(0));
      assertNull(decCol.getDecimal(1));

      assertTrue(i8Col.isNull(-1));
      assertTrue(i8Col.isNull(2));
      // setRowCount(2) must size the list child to rows * listSize = 4 elements.
      assertFalse(listChild.isNull(3));
      assertEquals(40, listChild.getInt(3));
      assertTrue(listChild.isNull(4));

      assertThrows(IllegalStateException.class, () -> f64Col.getSigned(0));
      assertThrows(IllegalStateException.class, () -> i64Col.getObject(0));
    }

    assertThrows(IllegalStateException.class, () -> boolCol.getBit(0));
    assertThrows(IllegalStateException.class, () -> listChild.getInt(0));
  }

  @Test
  void shouldMaterializeStructElementValuesAndClearMissingChildArrays() {
    Column boolChild = ColumnBuilder.of("b", PhysicalKind.BOOL, null);
    Column i8Child = ColumnBuilder.of("i8", PhysicalKind.INT8, null);
    Column i16Child = ColumnBuilder.of("i16", PhysicalKind.INT16, null);
    Column i32Child = ColumnBuilder.of("i32", PhysicalKind.INT32, null);
    Column d32Child = ColumnBuilder.of("d32", PhysicalKind.DATE32, null);
    Column i64Child = ColumnBuilder.of("i64", PhysicalKind.INT64, null);
    Column f32Child = ColumnBuilder.of("f32", PhysicalKind.FLOAT32, null);
    Column f64Child = ColumnBuilder.of("f64", PhysicalKind.FLOAT64, null);
    Column binChild = ColumnBuilder.of("bin", PhysicalKind.BINARY, null);
    Column decChild = ColumnBuilder.decimal("dec", 10, 2, null);

    ColumnBuilder.set(boolChild, 0, 1L);
    ColumnBuilder.set(i8Child, 0, 7L);
    ColumnBuilder.set(i16Child, 0, 300L);
    ColumnBuilder.set(i32Child, 0, 40_000L);
    ColumnBuilder.set(d32Child, 0, 100L);
    ColumnBuilder.set(i64Child, 0, 5_000_000_000L);
    ColumnBuilder.set(f32Child, 0, 2.5f);
    ColumnBuilder.set(f64Child, 0, 3.5d);
    ColumnBuilder.set(binChild, 0, new byte[] {9, 8});
    ColumnBuilder.set(decChild, 0, new BigDecimal("42.50"));

    try (Column structCol =
        ColumnBuilder.struct(
            "s", null, boolChild, i8Child, i16Child, i32Child, d32Child, i64Child, f32Child,
            f64Child, binChild, decChild)) {
      ColumnBuilder.setDefined(structCol, 0);
      ColumnBuilder.setNull(structCol, 1);

      Map<?, ?> row0 = (Map<?, ?>) structCol.getObject(0);
      assertEquals(Boolean.TRUE, row0.get("b"));
      assertEquals((byte) 7, row0.get("i8"));
      assertEquals((short) 300, row0.get("i16"));
      assertEquals(40_000, row0.get("i32"));
      assertEquals(100, row0.get("d32"));
      assertEquals(5_000_000_000L, row0.get("i64"));
      assertEquals(2.5f, row0.get("f32"));
      assertEquals(3.5d, row0.get("f64"));
      assertEquals(2, ((byte[]) row0.get("bin")).length);
      assertEquals(new BigDecimal("42.50"), row0.get("dec"));
      assertNull(structCol.getObject(1));

      structCol.bind(new NativeArrowArray(null, null, null, new NativeArrowArray[0]), 1);
      Map<?, ?> clearedRow = (Map<?, ?>) structCol.getObject(0);
      assertNull(clearedRow.get("i64"));
      assertNull(clearedRow.get("dec"));
    }
  }

  @Test
  void shouldRejectNegativeStringLengthWhenDecodingSchema() {
    ByteBuffer encoded = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(1).putInt(-1);

    assertThrows(IllegalArgumentException.class, () -> ColumnBatch.decodeSchema(encoded.array()));
  }

  @Test
  void shouldViewDecimalWithoutCopyingUnscaledBytesUntilRead() {
    BigDecimal value = new BigDecimal("1.00");
    ByteBuffer data = direct(16);
    byte[] unscaled = value.unscaledValue().toByteArray();
    for (int index = 0; index < unscaled.length; index++) {
      data.put(index, unscaled[unscaled.length - 1 - index]);
    }
    NativeArrowBatch batch = new NativeArrowBatch(1, new NativeArrowArray[] {array(data)});
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(field("d", PhysicalKind.DECIMAL128, null, 10, value.scale())));
    Column vector = ColumnBatch.view(schema, batch).column("d");
    assertEquals(value, vector.getDecimal(0));
  }

  @Test
  void shouldThrowWhenIsNullCalledOnClosedColumn() {
    Column column =
        ColumnBuilder.of("n", Column.PhysicalKind.INT32, Collections.<String, String>emptyMap());
    ColumnBuilder.set(column, 0, 1);
    assertTrue(column.isNull(1));
    column.close();
    assertThrows(IllegalStateException.class, () -> column.isNull(0));
  }

  @Test
  void shouldViewUtf8ColumnThroughNativeOffsetsWithoutCopyingValues() {
    NativeArrowBatch batch =
        new NativeArrowBatch(2, new NativeArrowArray[] {utf8Array("ab", "cdef")});
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(schemaBytes(field("label", PhysicalKind.UTF8, null)));

    ColumnBatch viewed = ColumnBatch.view(schema, batch);
    assertEquals(2, viewed.getRowCount());
    Column label = viewed.column("label");
    assertArrayEquals("ab".getBytes(StandardCharsets.UTF_8), label.getBytes(0));
    assertArrayEquals("cdef".getBytes(StandardCharsets.UTF_8), label.getBytes(1));
    assertFalse(label.isNull(0));
    assertFalse(label.isNull(1));
  }

  @Test
  void shouldViewStructWithBoolAndUtf8ChildrenFromNativeBuffers() {
    ByteBuffer boolValues = direct(1);
    boolValues.put(0, (byte) 1);
    NativeArrowBatch batch =
        new NativeArrowBatch(
            1, new NativeArrowArray[] {structArray(array(boolValues), utf8Array("xy"))});
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(
                structField(
                    "row",
                    field("active", PhysicalKind.BOOL, null),
                    field("name", PhysicalKind.UTF8, null))));

    Column root = ColumnBatch.view(schema, batch).column("row");
    Map<?, ?> value = (Map<?, ?>) root.getObject(0);
    assertEquals(true, value.get("active"));
    assertArrayEquals("xy".getBytes(StandardCharsets.UTF_8), (byte[]) value.get("name"));
  }

  @Test
  void shouldDecodeSchemaMetadataOntoFields() {
    Map<String, String> metadata = new LinkedHashMap<>();
    metadata.put("logicalType", "JSON");
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(fieldWithMetadata("payload", PhysicalKind.UTF8, metadata)));

    assertEquals(1, schema.size());
    assertEquals("payload", schema.get(0).getName());
    assertEquals(metadata, schema.get(0).getMetadata());
  }

  @Test
  void shouldRejectViewWhenSchemaColumnCountDiffersFromBatch() {
    NativeArrowBatch batch =
        new NativeArrowBatch(1, new NativeArrowArray[] {array(direct(8).putLong(0, 1))});
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(
                field("a", PhysicalKind.INT64, null), field("b", PhysicalKind.INT64, null)));

    assertThrows(IllegalArgumentException.class, () -> ColumnBatch.view(schema, batch));
  }

  @Test
  void shouldRejectUnknownPhysicalTagWhenDecodingSchema() {
    byte[] unknownTagField = fieldWithPhysicalTag("bad", 99);

    assertThrows(
        IllegalArgumentException.class,
        () -> ColumnBatch.decodeSchema(schemaBytes(unknownTagField)));
  }

  @Test
  void shouldRejectFixedSizeListLengthThatOverflowsInt() {
    NativeArrowBatch batch =
        new NativeArrowBatch(2, new NativeArrowArray[] {array(null, array(direct(8)))});
    List<Column.Field> schema =
        ColumnBatch.decodeSchema(
            schemaBytes(
                field(
                    "v",
                    PhysicalKind.FIXED_SIZE_LIST,
                    field("item", PhysicalKind.INT64, null),
                    Integer.MAX_VALUE)));

    SFSQLException failure =
        assertThrows(SFSQLException.class, () -> ColumnBatch.view(schema, batch));

    SQLException sql = failure.toSQLException();
    assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), sql.getErrorCode());
    assertEquals("Arrow list column exceeds the supported row count", failure.getMessage());
    assertSame(ArithmeticException.class, failure.getCause().getClass());
  }

  private static NativeArrowArray array(ByteBuffer values, NativeArrowArray... children) {
    return new NativeArrowArray(null, null, values, children);
  }

  private static NativeArrowArray structArray(NativeArrowArray... children) {
    return new NativeArrowArray(null, null, null, children);
  }

  private static NativeArrowArray utf8Array(String... values) {
    int rowCount = values.length;
    ByteBuffer offsets = direct((rowCount + 1) * Integer.BYTES);
    int length = 0;
    for (String value : values) {
      offsets.putInt(length);
      length += value.getBytes(StandardCharsets.UTF_8).length;
    }
    offsets.putInt(length);
    ByteBuffer data = direct(length);
    for (String value : values) {
      data.put(value.getBytes(StandardCharsets.UTF_8));
    }
    return new NativeArrowArray(null, offsets, data, new NativeArrowArray[0]);
  }

  private static byte[] structField(String name, byte[]... children) {
    ByteBuffer encoded = ByteBuffer.allocate(256).order(ByteOrder.LITTLE_ENDIAN);
    Arrays.stream(children).forEach(encoded::put);
    return field(
        name,
        PhysicalKind.STRUCT,
        Arrays.copyOf(encoded.array(), encoded.position()),
        children.length);
  }

  private static byte[] fieldWithMetadata(
      String name, PhysicalKind kind, Map<String, String> metadata) {
    byte[] nameBytes = name.getBytes(StandardCharsets.UTF_8);
    ByteBuffer encoded = ByteBuffer.allocate(128).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(nameBytes.length).put(nameBytes);
    encoded.putInt(metadata.size());
    for (Map.Entry<String, String> entry : metadata.entrySet()) {
      byte[] key = entry.getKey().getBytes(StandardCharsets.UTF_8);
      byte[] value = entry.getValue().getBytes(StandardCharsets.UTF_8);
      encoded.putInt(key.length).put(key).putInt(value.length).put(value);
    }
    encoded.put((byte) 1).put((byte) kind.getTag());
    return Arrays.copyOf(encoded.array(), encoded.position());
  }

  private static byte[] fieldWithPhysicalTag(String name, int tag) {
    byte[] nameBytes = name.getBytes(StandardCharsets.UTF_8);
    ByteBuffer encoded = ByteBuffer.allocate(64).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(nameBytes.length).put(nameBytes).putInt(0).put((byte) 1).put((byte) tag);
    return Arrays.copyOf(encoded.array(), encoded.position());
  }

  private static ByteBuffer direct(int bytes) {
    return ByteBuffer.allocateDirect(bytes).order(ByteOrder.LITTLE_ENDIAN);
  }

  private static byte[] schemaBytes(byte[]... fields) {
    ByteBuffer encoded = ByteBuffer.allocate(256).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(fields.length);
    Arrays.stream(fields).forEach(encoded::put);
    return Arrays.copyOf(encoded.array(), encoded.position());
  }

  /** Encodes a nullable field without metadata; {@code params} precede the optional child. */
  private static byte[] field(String name, PhysicalKind kind, byte[] child, int... params) {
    byte[] nameBytes = name.getBytes(StandardCharsets.UTF_8);
    ByteBuffer encoded = ByteBuffer.allocate(128).order(ByteOrder.LITTLE_ENDIAN);
    encoded.putInt(nameBytes.length).put(nameBytes).putInt(0).put((byte) 1);
    encoded.put((byte) kind.getTag());
    Arrays.stream(params).forEach(encoded::putInt);
    if (child != null) {
      encoded.put(child);
    }
    return Arrays.copyOf(encoded.array(), encoded.position());
  }
}
