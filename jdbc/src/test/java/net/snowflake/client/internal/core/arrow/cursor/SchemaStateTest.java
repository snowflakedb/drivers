package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.sql.Types;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
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
}
