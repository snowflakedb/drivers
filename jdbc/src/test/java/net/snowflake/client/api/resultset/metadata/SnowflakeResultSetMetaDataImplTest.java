package net.snowflake.client.api.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.sql.PreparedStatement;
import java.sql.ResultSetMetaData;
import java.sql.Types;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/** E2E coverage for the metadata returned by {@link PreparedStatement#getMetaData()}. */
class SnowflakeResultSetMetaDataImplTest extends SnowflakeIntegrationTestBase {
  // TODO(SNOW-3695645): cover all datatype here

  @Test
  void shouldDescribeColumnCountAndNames() throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT 1 AS a, 'x' AS b")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertEquals(2, meta.getColumnCount());
      assertEquals("A", meta.getColumnName(1));
      assertEquals("B", meta.getColumnName(2));
    }
  }

  @Test
  void shouldDescribeNumericColumnType() throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT 7::NUMBER(10,2) AS n")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertEquals(Types.DECIMAL, meta.getColumnType(1));
      assertEquals(10, meta.getPrecision(1));
      assertEquals(2, meta.getScale(1));
      assertTrue(meta.isSigned(1));
      assertEquals(BigDecimal.class.getName(), meta.getColumnClassName(1));
    }
  }

  @Test
  void shouldDescribeVarcharColumnType() throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT 'hello'::VARCHAR(20) AS s")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertEquals(Types.VARCHAR, meta.getColumnType(1));
      assertTrue(meta.isCaseSensitive(1));
      assertFalse(meta.isSigned(1));
      assertEquals(String.class.getName(), meta.getColumnClassName(1));
    }
  }

  @Test
  void shouldDescribeBooleanColumnType() throws Exception {
    try (PreparedStatement stmt = getDefaultConnection().prepareStatement("SELECT TRUE AS flag")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertEquals(Types.BOOLEAN, meta.getColumnType(1));
      assertFalse(meta.isCaseSensitive(1));
      assertEquals(Boolean.class.getName(), meta.getColumnClassName(1));
    }
  }

  @Test
  void shouldDescribeTimestampColumnType() throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT '2020-01-01'::TIMESTAMP_NTZ AS ts")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertEquals(Types.TIMESTAMP, meta.getColumnType(1));
    }
  }

  @Test
  void shouldReportColumnsReadOnlyAndNotWritable() throws Exception {
    try (PreparedStatement stmt = getDefaultConnection().prepareStatement("SELECT 1 AS a")) {
      ResultSetMetaData meta = stmt.getMetaData();

      assertTrue(meta.isReadOnly(1));
      assertFalse(meta.isWritable(1));
      assertFalse(meta.isDefinitelyWritable(1));
    }
  }
}
