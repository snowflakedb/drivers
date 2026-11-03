package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.sql.Types;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class SchemaStateTest {

  @Test
  public void testSchemaInitializationAndConverter() throws Exception {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TEXT");
    try (Column vector = ColumnBuilder.of("col_one", Column.PhysicalKind.UTF8, metadata);
        ColumnBatch root = ColumnBatch.of(vector)) {
      ColumnBuilder.set(vector, 0, "value".getBytes(StandardCharsets.UTF_8));
      ColumnBuilder.setValueCount(vector, 1);
      root.setRowCount(1);

      SchemaState schema = new SchemaState(root);
      assertArrayEquals(new String[] {"col_one"}, schema.getColumnNames());
      assertArrayEquals(new int[] {Types.VARCHAR}, schema.getColumnTypes());
      assertEquals(1, schema.getColumnCount());
      assertEquals("value", schema.getConverter(1, root).toString(0));

      // SchemaState is internal core-arrow plumbing (not a @JdbcBoundary); getConverter surfaces
      // the runtime carrier directly. The boundary translates it to a checked SQLException
      // upstream.
      SFSQLException exception =
          assertThrows(SFSQLException.class, () -> schema.getConverter(2, root));
      assertTrue(exception.getMessage().contains("Invalid column index"));
    }
  }

  @Test
  public void testDecfloatMapsToDecimalType() throws Exception {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "DECFLOAT");
    Column significand = ColumnBuilder.of("significand", Column.PhysicalKind.BINARY, null);
    Column exponent = ColumnBuilder.of("exponent", Column.PhysicalKind.INT16, null);
    try (Column vector = ColumnBuilder.struct("dec_col", metadata, significand, exponent);
        ColumnBatch root = ColumnBatch.of(vector)) {
      ColumnBuilder.set(significand, 0, new BigInteger("12345").toByteArray());
      ColumnBuilder.set(exponent, 0, (short) -2);
      ColumnBuilder.setDefined(vector, 0);
      ColumnBuilder.setValueCount(vector, 1);
      root.setRowCount(1);

      SchemaState schema = new SchemaState(root);
      assertArrayEquals(new String[] {"dec_col"}, schema.getColumnNames());
      assertArrayEquals(new int[] {Types.DECIMAL}, schema.getColumnTypes());
      assertEquals(1, schema.getColumnCount());
      assertEquals(new BigDecimal("123.45"), schema.getConverter(1, root).toBigDecimal(0));
    }
  }

  @Test
  public void shouldReadColumnScalesTimeStringLengthAndWrapConverterInitFailure() throws Exception {
    Map<String, String> validScaleMeta = new HashMap<>();
    validScaleMeta.put("logicalType", "FIXED");
    validScaleMeta.put("scale", "4");

    Map<String, String> invalidScaleMeta = new HashMap<>();
    invalidScaleMeta.put("logicalType", "FIXED");
    invalidScaleMeta.put("scale", "not-a-scale");

    try (Column validCol = ColumnBuilder.of("c1", Column.PhysicalKind.INT32, validScaleMeta);
        Column invalidScaleCol =
            ColumnBuilder.of("c2", Column.PhysicalKind.INT32, invalidScaleMeta);
        Column float32Col = ColumnBuilder.of("c3", Column.PhysicalKind.FLOAT32, null);
        ColumnBatch root = ColumnBatch.of(validCol, invalidScaleCol, float32Col)) {
      SchemaState schema = new SchemaState(root);

      assertArrayEquals(new int[] {4, 0, 0}, schema.getColumnScales());
      assertEquals(8, schema.getTimeStringLength());
      assertFalse(schema.hasCachedConverter(1));
      assertFalse(schema.hasCachedConverter(0));
      assertFalse(schema.hasCachedConverter(99));

      assertSame(schema.getConverter(1, root), schema.getConverter(1, root));
      assertTrue(schema.hasCachedConverter(1));

      SFSQLException initError =
          assertThrows(SFSQLException.class, () -> schema.getConverter(3, root));
      assertEquals(ErrorCode.INTERNAL_ERROR, initError.getErrorCode());
      assertTrue(initError.getMessage().contains("Unable to create converter for column 3"));

      schema.reset();
      assertFalse(schema.hasCachedConverter(1));
    }
  }

  @Test
  public void shouldDropAllCachedConvertersWhenAskedForDifferentRootInstance() throws Exception {
    Map<String, String> textMeta = Collections.singletonMap("logicalType", "TEXT");
    try (Column firstCol1 = ColumnBuilder.of("c1", Column.PhysicalKind.UTF8, textMeta);
        Column firstCol2 = ColumnBuilder.of("c2", Column.PhysicalKind.UTF8, textMeta);
        ColumnBatch firstRoot = ColumnBatch.of(firstCol1, firstCol2);
        Column secondCol1 = ColumnBuilder.of("c1", Column.PhysicalKind.UTF8, textMeta);
        Column secondCol2 = ColumnBuilder.of("c2", Column.PhysicalKind.UTF8, textMeta);
        ColumnBatch secondRoot = ColumnBatch.of(secondCol1, secondCol2)) {
      ColumnBuilder.set(firstCol1, 0, "alpha".getBytes(StandardCharsets.UTF_8));
      ColumnBuilder.setValueCount(firstCol1, 1);
      ColumnBuilder.setValueCount(firstCol2, 1);
      firstRoot.setRowCount(1);
      ColumnBuilder.set(secondCol1, 0, "gamma".getBytes(StandardCharsets.UTF_8));
      ColumnBuilder.setValueCount(secondCol1, 1);
      ColumnBuilder.setValueCount(secondCol2, 1);
      secondRoot.setRowCount(1);

      SchemaState schema = new SchemaState(firstRoot);
      ArrowVectorConverter firstC1 = schema.getConverter(1, firstRoot);
      schema.getConverter(2, firstRoot);
      assertEquals("alpha", firstC1.toString(0));

      ArrowVectorConverter secondC1 = schema.getConverter(1, secondRoot);

      assertNotSame(firstC1, secondC1);
      assertEquals("gamma", secondC1.toString(0));
      assertTrue(schema.hasCachedConverter(1));
      assertFalse(schema.hasCachedConverter(2));
    }
  }
}
