package net.snowflake.client.api.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertAll;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.sql.Types;
import java.util.List;
import java.util.UUID;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/** Tests ResultSetMetaData for structured types (OBJECT, ARRAY, MAP) and VECTOR columns. */
class SnowflakeResultSetMetaDataImplStructuredTypesTest extends SnowflakeIntegrationTestBase
    implements WithMetaDataAssertions {

  private Connection openStructuredTypesConnection() throws Exception {
    Connection conn = openConnection();
    try (Statement stmt = conn.createStatement()) {
      stmt.execute("ALTER SESSION SET ENABLE_STRUCTURED_TYPES_IN_CLIENT_RESPONSE = TRUE");
      stmt.execute("ALTER SESSION SET IGNORE_CLIENT_VESRION_IN_STRUCTURED_TYPES_RESPONSE = TRUE");
    }
    return conn;
  }

  @Test
  @SkipNewDriver("not yet implemented - structured types field metadata")
  void shouldDescribeStructuredObjectFieldMetadata() throws Exception {
    try (Connection conn = openStructuredTypesConnection();
        Statement stmt = conn.createStatement();
        java.sql.ResultSet rs =
            stmt.executeQuery("SELECT {'a': 1, 'b': 'x'}::OBJECT(a INTEGER, b VARCHAR) AS col")) {
      SnowflakeResultSetMetaData meta = rs.getMetaData().unwrap(SnowflakeResultSetMetaData.class);

      assertEquals(Types.STRUCT, rs.getMetaData().getColumnType(1));
      assertEquals("OBJECT", rs.getMetaData().getColumnTypeName(1));

      List<FieldMetadata> fields = meta.getColumnFields(1);
      assertEquals(2, fields.size());
      // INTEGER inside a structured type is FIXED-backed, reported as BIGINT / "NUMBER".
      assertAll(
          "object fields",
          () -> assertEquals("a", fields.get(0).getName()),
          () -> assertEquals(Types.BIGINT, fields.get(0).getType()),
          () -> assertEquals("NUMBER", fields.get(0).getTypeName()),
          () -> assertEquals(SnowflakeType.FIXED, fields.get(0).getBase()),
          () -> assertEquals("b", fields.get(1).getName()),
          () -> assertEquals(Types.VARCHAR, fields.get(1).getType()),
          () -> assertEquals("VARCHAR", fields.get(1).getTypeName()),
          () -> assertEquals(SnowflakeType.TEXT, fields.get(1).getBase()));
    }
  }

  @Test
  @SkipNewDriver("not yet implemented - structured types field metadata")
  void shouldDescribeStructuredArrayFieldMetadata() throws Exception {
    try (Connection conn = openStructuredTypesConnection();
        Statement stmt = conn.createStatement();
        java.sql.ResultSet rs =
            stmt.executeQuery("SELECT ARRAY_CONSTRUCT(1, 2)::ARRAY(INTEGER) AS col")) {
      SnowflakeResultSetMetaData meta = rs.getMetaData().unwrap(SnowflakeResultSetMetaData.class);

      assertEquals(Types.ARRAY, rs.getMetaData().getColumnType(1));
      assertEquals("ARRAY", rs.getMetaData().getColumnTypeName(1));

      // A structured ARRAY exposes a single field describing its element type.
      List<FieldMetadata> fields = meta.getColumnFields(1);
      assertEquals(1, fields.size());
      assertAll(
          "array element field",
          () -> assertEquals(Types.BIGINT, fields.get(0).getType()),
          () -> assertEquals("NUMBER", fields.get(0).getTypeName()),
          () -> assertEquals(SnowflakeType.FIXED, fields.get(0).getBase()));
    }
  }

  @Test
  @SkipNewDriver("not yet implemented - structured types field metadata")
  void shouldDescribeStructuredMapFieldMetadata() throws Exception {
    try (Connection conn = openStructuredTypesConnection();
        Statement stmt = conn.createStatement();
        java.sql.ResultSet rs =
            stmt.executeQuery("SELECT OBJECT_CONSTRUCT('k', 1)::MAP(VARCHAR, INTEGER) AS col")) {
      SnowflakeResultSetMetaData meta = rs.getMetaData().unwrap(SnowflakeResultSetMetaData.class);

      assertEquals(Types.STRUCT, rs.getMetaData().getColumnType(1));

      // A structured MAP exposes two fields: key type then value type.
      List<FieldMetadata> fields = meta.getColumnFields(1);
      assertEquals(2, fields.size());
      assertAll(
          "map key/value fields",
          () -> assertEquals(Types.VARCHAR, fields.get(0).getType()),
          () -> assertEquals(SnowflakeType.TEXT, fields.get(0).getBase()),
          () -> assertEquals(Types.BIGINT, fields.get(1).getType()),
          () -> assertEquals(SnowflakeType.FIXED, fields.get(1).getBase()));
    }
  }

  @Test
  void shouldDescribeVectorColumnMetadata() throws Exception {
    String table = "TEST_VECTOR_META_" + UUID.randomUUID().toString().replace("-", "");
    try (Connection conn = openConnection();
        Statement ddl = conn.createStatement()) {
      ddl.execute("CREATE TEMP TABLE " + table + " (col VECTOR(INT, 3))");
      try (PreparedStatement stmt = conn.prepareStatement("SELECT col FROM " + table)) {
        ResultSetMetaData meta = stmt.getMetaData();

        assertEquals(SnowflakeType.EXTRA_TYPES_VECTOR, meta.getColumnType(1));
        assertEquals("VECTOR", meta.getColumnTypeName(1));
        assertFalse(meta.isSigned(1));
        assertFalse(meta.isCaseSensitive(1));
        assertEquals(3, meta.unwrap(SnowflakeResultSetMetaData.class).getVectorDimension(1));
      }
    }
  }
}
