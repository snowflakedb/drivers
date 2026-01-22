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

      SchemaState schema = new SchemaState();
      assertArrayEquals(new String[] {"col_one"}, schema.getColumnNames(root));
      assertArrayEquals(new int[] {Types.VARCHAR}, schema.getColumnTypes(root));
      assertEquals(1, schema.getColumnCount(root));
      assertNotNull(schema.getConverter(1, root));

      SQLException exception = assertThrows(SQLException.class, () -> schema.getConverter(2, root));
      assertTrue(exception.getMessage().contains("Invalid column index"));
    }
  }
}
