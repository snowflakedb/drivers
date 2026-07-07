package net.snowflake.client.api.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.math.BigDecimal;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.sql.Types;
import java.util.UUID;
import java.util.stream.Stream;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/** Tests ResultSetMetaData for scalar types and JDBC_TREAT_DECIMAL_AS_INT behavior. */
class SnowflakeResultSetMetaDataImplTest extends SnowflakeIntegrationTestBase
    implements WithMetaDataAssertions {

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

  static Stream<Arguments> scalarDatatypeCases() {
    return Stream.of(
        // sql, jdbcType, typeName, className, precision, scale, displaySize, signed, caseSensitive
        Arguments.of(
            "7::NUMBER(10,2)",
            Types.DECIMAL,
            "NUMBER",
            BigDecimal.class,
            10,
            2,
            12, // precision + 1 (sign) + 1 (decimal point)
            true,
            false),
        Arguments.of(
            "1.5::FLOAT",
            Types.DOUBLE,
            "DOUBLE",
            Double.class,
            0,
            0,
            24, // hard-coded double display size
            true,
            false),
        Arguments.of("1.5::DOUBLE", Types.DOUBLE, "DOUBLE", Double.class, 0, 0, 24, true, false),
        Arguments.of(
            "'1.5'::DECFLOAT", Types.DECIMAL, "DECFLOAT", BigDecimal.class, 38, 0, 40, true, false),
        Arguments.of(
            "'hello'::VARCHAR(20)",
            Types.VARCHAR,
            "VARCHAR",
            String.class,
            20, // precision == declared length
            0,
            20, // display size == length
            false,
            true),
        Arguments.of(
            "'hello'::VARCHAR",
            Types.VARCHAR,
            "VARCHAR",
            String.class,
            16 * 1024 * 1024 * 8, // 16 MB max VARCHAR length
            0,
            16 * 1024 * 1024 * 8,
            false,
            true),
        // CHAR is reported as VARCHAR by the driver.
        Arguments.of("'c'::CHAR(1)", Types.VARCHAR, "VARCHAR", String.class, 1, 0, 1, false, true),
        Arguments.of(
            "TO_BINARY('AB', 'HEX')::BINARY(8)",
            Types.BINARY,
            "BINARY",
            byte[].class,
            8, // precision == declared byte length
            0,
            8,
            false,
            false),
        Arguments.of(
            "TRUE::BOOLEAN",
            Types.BOOLEAN,
            "BOOLEAN",
            Boolean.class,
            0,
            0,
            5, // hard-coded "false".length()
            false,
            false),
        Arguments.of(
            "PARSE_JSON('{\"a\":1}')::VARIANT",
            Types.VARCHAR,
            "VARIANT",
            String.class,
            0,
            0,
            0,
            false,
            true),
        Arguments.of(
            "OBJECT_CONSTRUCT('a', 1)::OBJECT",
            Types.VARCHAR, // semi-structured (untyped) OBJECT is reported as VARCHAR
            "OBJECT",
            String.class,
            0,
            0,
            0,
            false,
            true),
        Arguments.of(
            "ARRAY_CONSTRUCT(1, 2)::ARRAY",
            Types.VARCHAR, // semi-structured (untyped) ARRAY is reported as VARCHAR
            "ARRAY",
            String.class,
            0,
            0,
            0,
            false,
            true),
        Arguments.of(
            "TO_GEOGRAPHY('POINT(1 1)')",
            Types.VARCHAR,
            "GEOGRAPHY",
            String.class,
            0,
            0,
            0,
            false,
            true),
        Arguments.of(
            "TO_GEOMETRY('POINT(1 1)')",
            Types.VARCHAR,
            "GEOMETRY",
            String.class,
            0,
            0,
            0,
            false,
            true));
  }

  @ParameterizedTest(name = "should describe metadata for {0}")
  @MethodSource("scalarDatatypeCases")
  void shouldDescribeMetadataForScalarDatatypes(
      String sqlExpression,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive)
      throws Exception {
    try (PreparedStatement stmt =
        getDefaultConnection().prepareStatement("SELECT " + sqlExpression + " AS col")) {
      assertColumnMetadata(
          stmt.getMetaData(),
          sqlExpression,
          expectedType,
          expectedTypeName,
          expectedClass,
          expectedPrecision,
          expectedScale,
          expectedDisplaySize,
          expectedSigned,
          expectedCaseSensitive);
    }
  }

  static Stream<Arguments> treatDecimalAsIntTrueCases() {
    return Stream.of(
        // sql, jdbcType, typeName, className, precision, scale, displaySize, signed, caseSensitive
        Arguments.of("1::INT", Types.BIGINT, "NUMBER", Long.class, 38, 0, 39, true, false),
        Arguments.of("1::NUMBER(10,0)", Types.BIGINT, "NUMBER", Long.class, 10, 0, 11, true, false),
        Arguments.of("1::BIGINT", Types.BIGINT, "NUMBER", Long.class, 38, 0, 39, true, false),
        Arguments.of("1::SMALLINT", Types.BIGINT, "NUMBER", Long.class, 38, 0, 39, true, false),
        Arguments.of("1::TINYINT", Types.BIGINT, "NUMBER", Long.class, 38, 0, 39, true, false),
        Arguments.of("1::INTEGER", Types.BIGINT, "NUMBER", Long.class, 38, 0, 39, true, false));
  }

  @SkipNewDriver("not yet implemented - handling JDBC_TREAT_DECIMAL_AS_INT")
  @ParameterizedTest(name = "should report BIGINT when JDBC_TREAT_DECIMAL_AS_INT=true for {0}")
  @MethodSource("treatDecimalAsIntTrueCases")
  void shouldReportBigintWhenTreatDecimalAsIntTrue(
      String sqlExpression,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive)
      throws Exception {
    try (Connection conn = openConnection()) {
      try (Statement alter = conn.createStatement()) {
        alter.execute("ALTER SESSION SET JDBC_TREAT_DECIMAL_AS_INT = true");
      }
      try (PreparedStatement stmt = conn.prepareStatement("SELECT " + sqlExpression + " AS col")) {
        assertColumnMetadata(
            stmt.getMetaData(),
            sqlExpression,
            expectedType,
            expectedTypeName,
            expectedClass,
            expectedPrecision,
            expectedScale,
            expectedDisplaySize,
            expectedSigned,
            expectedCaseSensitive);
      }
    }
  }

  static Stream<Arguments> treatDecimalAsIntFalseCases() {
    return Stream.of(
        Arguments.of("1::INT", Types.DECIMAL, "NUMBER", BigDecimal.class, 38, 0, 40, true, false),
        Arguments.of(
            "1::NUMBER(10,0)", Types.DECIMAL, "NUMBER", BigDecimal.class, 10, 0, 12, true, false),
        Arguments.of(
            "1::BIGINT", Types.DECIMAL, "NUMBER", BigDecimal.class, 38, 0, 40, true, false),
        Arguments.of(
            "1::SMALLINT", Types.DECIMAL, "NUMBER", BigDecimal.class, 38, 0, 40, true, false),
        Arguments.of(
            "1::TINYINT", Types.DECIMAL, "NUMBER", BigDecimal.class, 38, 0, 40, true, false),
        Arguments.of(
            "1::INTEGER", Types.DECIMAL, "NUMBER", BigDecimal.class, 38, 0, 40, true, false));
  }

  @ParameterizedTest(name = "should report DECIMAL when JDBC_TREAT_DECIMAL_AS_INT=false for {0}")
  @MethodSource("treatDecimalAsIntFalseCases")
  void shouldReportDecimalWhenTreatDecimalAsIntFalse(
      String sqlExpression,
      int expectedType,
      String expectedTypeName,
      Class<?> expectedClass,
      int expectedPrecision,
      int expectedScale,
      int expectedDisplaySize,
      boolean expectedSigned,
      boolean expectedCaseSensitive)
      throws Exception {
    try (Connection conn = openConnection()) {
      try (Statement alter = conn.createStatement()) {
        alter.execute("ALTER SESSION SET JDBC_TREAT_DECIMAL_AS_INT = false");
      }
      try (PreparedStatement stmt = conn.prepareStatement("SELECT " + sqlExpression + " AS col")) {
        assertColumnMetadata(
            stmt.getMetaData(),
            sqlExpression,
            expectedType,
            expectedTypeName,
            expectedClass,
            expectedPrecision,
            expectedScale,
            expectedDisplaySize,
            expectedSigned,
            expectedCaseSensitive);
      }
    }
  }

  @Test
  void shouldReportCatalogSchemaAndTableName() throws Exception {
    String table = "TEST_SRC_META_" + UUID.randomUUID().toString().replace("-", "").toUpperCase();
    try (Connection conn = openConnection();
        Statement ddl = conn.createStatement()) {
      ddl.execute("CREATE TEMP TABLE " + table + " (x INT)");
      try (PreparedStatement stmt = conn.prepareStatement("SELECT x FROM " + table)) {
        ResultSetMetaData meta = stmt.getMetaData();

        assertEquals(conn.getCatalog(), meta.getCatalogName(1), "catalog (database)");
        assertEquals(conn.getSchema(), meta.getSchemaName(1), "schema");
        assertEquals(table, meta.getTableName(1), "table");
      }
    }
  }
}
