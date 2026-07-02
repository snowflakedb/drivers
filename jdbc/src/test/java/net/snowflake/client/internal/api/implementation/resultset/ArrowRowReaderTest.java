package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.VarCharVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.ipc.ArrowStreamReader;
import org.apache.arrow.vector.ipc.ArrowStreamWriter;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

class ArrowRowReaderTest {

  private static ArrowResources buildResources(
      BufferAllocator allocator, int[][] ids, String[][] names) throws IOException {
    Map<String, String> intMeta = new HashMap<>();
    intMeta.put("logicalType", "FIXED");
    intMeta.put("precision", "10");
    intMeta.put("scale", "0");
    Map<String, String> textMeta = new HashMap<>();
    textMeta.put("logicalType", "TEXT");

    FieldType intType = new FieldType(true, Types.MinorType.INT.getType(), null, intMeta);
    FieldType textType = new FieldType(true, Types.MinorType.VARCHAR.getType(), null, textMeta);

    ByteArrayOutputStream out = new ByteArrayOutputStream();
    try (IntVector idVec = new IntVector("id", intType, allocator);
        VarCharVector nameVec = new VarCharVector("name", textType, allocator);
        VectorSchemaRoot root = VectorSchemaRoot.of(idVec, nameVec);
        ArrowStreamWriter writer = new ArrowStreamWriter(root, null, out)) {
      writer.start();
      for (int b = 0; b < ids.length; b++) {
        int rows = ids[b].length;
        idVec.allocateNew(rows);
        nameVec.allocateNew(rows);
        for (int r = 0; r < rows; r++) {
          idVec.setSafe(r, ids[b][r]);
          nameVec.setSafe(r, names[b][r].getBytes(StandardCharsets.UTF_8));
        }
        idVec.setValueCount(rows);
        nameVec.setValueCount(rows);
        root.setRowCount(rows);
        writer.writeBatch();
        idVec.clear();
        nameVec.clear();
      }
      writer.end();
    }

    ArrowStreamReader reader =
        new ArrowStreamReader(new ByteArrayInputStream(out.toByteArray()), allocator);
    return new ArrowResources(null, allocator, reader);
  }

  // --- cursor navigation ---

  @Test
  void shouldIterateSingleBatch() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res =
          buildResources(alloc, new int[][] {{1, 2, 3}}, new String[][] {{"a", "b", "c"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        assertTrue(reader.isBeforeFirst());
        assertFalse(reader.isAfterLast());

        assertTrue(reader.next());
        assertTrue(reader.isFirst());
        assertEquals(0, reader.getCurrentRow());
        assertEquals("a", reader.getString(2));
        assertEquals(1, reader.getInt(1));

        assertTrue(reader.next());
        assertFalse(reader.isFirst());
        assertEquals(1, reader.getCurrentRow());
        assertEquals("b", reader.getString(2));

        assertTrue(reader.next());
        assertEquals(2, reader.getCurrentRow());
        assertEquals("c", reader.getString(2));
        assertEquals(3, reader.getInt(1));

        assertFalse(reader.next());
        assertTrue(reader.isAfterLast());
      }
    }
  }

  @Test
  void shouldIterateMultipleBatches() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res =
          buildResources(alloc, new int[][] {{10}, {20, 30}}, new String[][] {{"x"}, {"y", "z"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        assertTrue(reader.next());
        assertEquals(10, reader.getInt(1));

        assertTrue(reader.next());
        assertEquals(20, reader.getInt(1));

        assertTrue(reader.next());
        assertEquals(30, reader.getInt(1));

        assertFalse(reader.next());
      }
    }
  }

  // --- typed getters ---

  @Test
  void shouldReturnIntAsString() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{42}}, new String[][] {{"hi"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.next();
        assertEquals("42", reader.getString(1));
        assertEquals("hi", reader.getString(2));
      }
    }
  }

  @Test
  void shouldReturnNumericValuesFromIntColumn() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{7}}, new String[][] {{"ignored"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.next();
        assertEquals((byte) 7, reader.getByte(1));
        assertEquals((short) 7, reader.getShort(1));
        assertEquals(7, reader.getInt(1));
        assertEquals(7L, reader.getLong(1));
        assertEquals(7.0f, reader.getFloat(1));
        assertEquals(7.0, reader.getDouble(1));
      }
    }
  }

  @Test
  void shouldReturnBooleanFromIntColumn() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{0, 1}}, new String[][] {{"a", "b"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.next();
        assertFalse(reader.getBoolean(1));

        reader.next();
        assertTrue(reader.getBoolean(1));
      }
    }
  }

  // --- column metadata ---

  @Test
  void shouldReturnColumnCountAndNames() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        assertEquals(2, reader.getColumnCount());
        assertEquals("id", reader.getColumnName(1));
        assertEquals("name", reader.getColumnName(2));
      }
    }
  }

  @Test
  void shouldThrowOnColumnIndexOutOfRange() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        assertThrows(SQLException.class, () -> reader.getColumnName(0));
        assertThrows(SQLException.class, () -> reader.getColumnName(3));
      }
    }
  }

  // --- state checks ---

  @Test
  void shouldThrowOnGetterBeforeNext() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        assertThrows(SQLException.class, () -> reader.getInt(1));
      }
    }
  }

  @Test
  void shouldThrowOnGetterAfterExhausted() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.next();
        reader.next();
        assertTrue(reader.isAfterLast());
        assertThrows(SQLException.class, () -> reader.getInt(1));
      }
    }
  }

  @Test
  void shouldReturnFalseOnNextAfterClose() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.close();
        assertTrue(reader.isClosed());
        assertFalse(reader.next());
      }
    }
  }

  @Test
  void shouldCloseIdempotently() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{1}}, new String[][] {{"a"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.close();
        reader.close();
        assertTrue(reader.isClosed());
      }
    }
  }

  // --- wasNull ---

  @Test
  void shouldReportNonNullAfterNonNullGet() throws Exception {
    try (BufferAllocator alloc = new RootAllocator(Long.MAX_VALUE)) {
      ArrowResources res = buildResources(alloc, new int[][] {{5}}, new String[][] {{"val"}});
      try (ArrowRowReader reader = new ArrowRowReader(res, new DataContextStub())) {
        reader.next();
        reader.getInt(1);
        assertFalse(reader.wasNull());

        reader.getString(2);
        assertFalse(reader.wasNull());
      }
    }
  }

  private static class DataContextStub implements DataConversionContext {}
}
