package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.sql.Types;
import java.util.HashMap;
import java.util.Map;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.VarCharVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.types.Types.MinorType;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;

public class SchemaStateTest {

  @Test
  public void testSchemaInitializationAndConverter() throws Exception {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TEXT");
    FieldType fieldType = new FieldType(true, MinorType.VARCHAR.getType(), null, metadata);

    try (RootAllocator allocator = new RootAllocator(Long.MAX_VALUE);
        VarCharVector vector = new VarCharVector("col_one", fieldType, allocator);
        VectorSchemaRoot root = VectorSchemaRoot.of(vector)) {
      vector.allocateNew();
      vector.setSafe(0, "value".getBytes(StandardCharsets.UTF_8));
      vector.setValueCount(1);
      root.setRowCount(1);

      SchemaState schema = new SchemaState(root);
      assertArrayEquals(new String[] {"col_one"}, schema.getColumnNames());
      assertArrayEquals(new int[] {Types.VARCHAR}, schema.getColumnTypes());
      assertEquals(1, schema.getColumnCount());
      assertNotNull(schema.getConverter(1, root));

      SQLException exception = assertThrows(SQLException.class, () -> schema.getConverter(2, root));
      assertTrue(exception.getMessage().contains("Invalid column index"));
    }
  }

  @Test
  public void testDecfloatMapsToDecimalType() throws Exception {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "DECFLOAT");
    FieldType fieldType = new FieldType(true, MinorType.VARCHAR.getType(), null, metadata);

    try (RootAllocator allocator = new RootAllocator(Long.MAX_VALUE);
        VarCharVector vector = new VarCharVector("dec_col", fieldType, allocator);
        VectorSchemaRoot root = VectorSchemaRoot.of(vector)) {
      vector.allocateNew();
      vector.setSafe(0, "123.45".getBytes(StandardCharsets.UTF_8));
      vector.setValueCount(1);
      root.setRowCount(1);

      SchemaState schema = new SchemaState(root);
      assertArrayEquals(new String[] {"dec_col"}, schema.getColumnNames());
      assertArrayEquals(new int[] {Types.DECIMAL}, schema.getColumnTypes());
      assertEquals(1, schema.getColumnCount());
    }
  }
}
