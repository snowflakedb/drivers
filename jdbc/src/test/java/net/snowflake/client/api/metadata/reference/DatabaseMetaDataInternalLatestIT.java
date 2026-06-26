package net.snowflake.client.api.metadata.reference;

import static net.snowflake.client.api.metadata.reference.DatabaseMetaDataInternalIT.endMetaData;
import static net.snowflake.client.api.metadata.reference.DatabaseMetaDataInternalIT.initMetaData;
import static net.snowflake.client.api.metadata.reference.DatabaseMetaDataInternalIT.jdbcDb1;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * Database Metadata tests for the latest JDBC driver. This doesn't work for the oldest supported
 * driver. Revisit this tests whenever bumping up the oldest supported driver to examine if the
 * tests still is not applicable. If it is applicable, move tests to DatabaseMetaDataIT so that both
 * the latest and oldest supported driver run the tests.
 */
public class DatabaseMetaDataInternalLatestIT extends SnowflakeIntegrationTestBase {

  @BeforeEach
  public void setUp() throws Exception {
    try (Connection con = openConnection()) {
      initMetaData(con);
    }
  }

  @AfterEach
  public void tearDown() throws Exception {
    try (Connection con = openConnection()) {
      endMetaData(con);
    }
  }

  @Test
  @SkipNewDriver("not yet implemented")
  public void testGetMetaDataUseConnectionCtx() throws SQLException {
    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {

      // setup: reset session db and schema, enable the parameter
      statement.execute("use database " + jdbcDb1);
      statement.execute("use schema JDBC_SCHEMA11");
      statement.execute("alter SESSION set CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX=true");

      DatabaseMetaData databaseMetaData = connection.getMetaData();

      try (ResultSet resultSet = databaseMetaData.getTables(null, null, null, null)) {
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString(1));
        assertEquals("JDBC_SCHEMA11", resultSet.getString(2));
      }
      try (ResultSet resultSet = databaseMetaData.getColumns(null, null, null, null); ) {
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString(1));
        assertEquals("JDBC_SCHEMA11", resultSet.getString(2));
      }
    }
  }

  @Test
  @SkipNewDriver("not yet implemented")
  public void testGetFunctionColumns() throws SQLException {
    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {
      statement.execute(
          "create or replace function "
              + jdbcDb1
              + ".JDBC_SCHEMA11.FUNC111 "
              + "(a number, b number) RETURNS NUMBER COMMENT='multiply numbers' as 'a*b'");
      statement.execute(
          "create or replace table "
              + jdbcDb1
              + ".JDBC_SCHEMA11.BIN_TABLE(bin1 binary, bin2 binary(100), "
              + "sharedCol decimal)");
      statement.execute(
          "create or replace function "
              + jdbcDb1
              + ".JDBC_SCHEMA11.FUNC112 "
              + "() RETURNS TABLE(colA string(16777216), colB decimal, bin2 binary(8388608), sharedCol decimal) COMMENT= 'returns "
              + "table of 4 columns'"
              + " as 'select "
              + jdbcDb1
              + ".JDBC_SCHEMA11.JDBC_TBL111.colA, "
              + jdbcDb1
              + ".JDBC_SCHEMA11.JDBC_TBL111.colB, "
              + jdbcDb1
              + ".JDBC_SCHEMA11.BIN_TABLE.bin2, "
              + jdbcDb1
              + ".JDBC_SCHEMA11.BIN_TABLE.sharedCol from "
              + jdbcDb1
              + ".JDBC_SCHEMA11.JDBC_TBL111 inner join "
              + jdbcDb1
              + ".JDBC_SCHEMA11.BIN_TABLE on "
              + jdbcDb1
              + ".JDBC_SCHEMA11.JDBC_TBL111.colB = "
              + jdbcDb1
              + ".JDBC_SCHEMA11.BIN_TABLE.sharedCol'");
      DatabaseMetaData databaseMetaData = connection.getMetaData();
      try (ResultSet resultSet =
          databaseMetaData.getFunctionColumns(jdbcDb1, "JDBC_SCHEMA11", "FUNC111", "%")) {
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC111", resultSet.getString("FUNCTION_NAME"));
        assertEquals("", resultSet.getString("COLUMN_NAME"));
        assertEquals(DatabaseMetaData.functionReturn, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.NUMERIC, resultSet.getInt("DATA_TYPE"));
        assertEquals("NUMBER(38,0)", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getShort("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("multiply numbers", resultSet.getString("REMARKS"));
        assertEquals(0, resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(0, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertEquals("FUNC111(NUMBER, NUMBER) RETURN NUMBER", resultSet.getString("SPECIFIC_NAME"));
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC111", resultSet.getString("FUNCTION_NAME"));
        assertEquals("A", resultSet.getString("COLUMN_NAME"));
        assertEquals(1, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.NUMERIC, resultSet.getInt("DATA_TYPE"));
        assertEquals("NUMBER", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getShort("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("multiply numbers", resultSet.getString("REMARKS"));
        assertEquals(0, resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(1, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertEquals("FUNC111(NUMBER, NUMBER) RETURN NUMBER", resultSet.getString("SPECIFIC_NAME"));
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC111", resultSet.getString("FUNCTION_NAME"));
        assertEquals("B", resultSet.getString("COLUMN_NAME"));
        assertEquals(1, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.NUMERIC, resultSet.getInt("DATA_TYPE"));
        assertEquals("NUMBER", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getShort("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("multiply numbers", resultSet.getString("REMARKS"));
        assertEquals(0, resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(2, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertEquals("FUNC111(NUMBER, NUMBER) RETURN NUMBER", resultSet.getString("SPECIFIC_NAME"));
        assertFalse(resultSet.next());
      }
      try (ResultSet resultSet =
          databaseMetaData.getFunctionColumns(jdbcDb1, "JDBC_SCHEMA11", "FUNC112", "%")) {
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC112", resultSet.getString("FUNCTION_NAME"));
        assertEquals("COLA", resultSet.getString("COLUMN_NAME"));
        assertEquals(DatabaseMetaData.functionColumnResult, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.VARCHAR, resultSet.getInt("DATA_TYPE"));
        assertEquals("VARCHAR", resultSet.getString("TYPE_NAME"));
        assertEquals(0, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getInt("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("returns table of 4 columns", resultSet.getString("REMARKS"));
        assertEquals(
            databaseMetaData.getMaxCharLiteralLength(), resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(1, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertTrue(
            resultSet
                .getString("SPECIFIC_NAME")
                .replaceAll("\\s", "")
                .matches(
                    "^FUNC112.*RETURNTABLE.*COLAVARCHAR.*,COLBNUMBER,BIN2BINARY.*,SHAREDCOLNUMBER.?$"),
            "Columns metadata SPECIFIC_NAME should contains expected columns ");
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC112", resultSet.getString("FUNCTION_NAME"));
        assertEquals("COLB", resultSet.getString("COLUMN_NAME"));
        assertEquals(DatabaseMetaData.functionColumnResult, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.NUMERIC, resultSet.getInt("DATA_TYPE"));
        assertEquals("NUMBER", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getInt("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("returns table of 4 columns", resultSet.getString("REMARKS"));
        assertEquals(0, resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(2, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertTrue(
            resultSet
                .getString("SPECIFIC_NAME")
                .replaceAll("\\s", "")
                .matches(
                    "^FUNC112.*RETURNTABLE.*COLAVARCHAR.*,COLBNUMBER,BIN2BINARY.*,SHAREDCOLNUMBER.?$"),
            "Columns metadata SPECIFIC_NAME should contains expected columns ");
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC112", resultSet.getString("FUNCTION_NAME"));
        assertEquals("BIN2", resultSet.getString("COLUMN_NAME"));
        assertEquals(DatabaseMetaData.functionColumnResult, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.BINARY, resultSet.getInt("DATA_TYPE"));
        assertEquals("BINARY", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getInt("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("returns table of 4 columns", resultSet.getString("REMARKS"));
        assertEquals(
            databaseMetaData.getMaxBinaryLiteralLength(), resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(3, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertTrue(
            resultSet
                .getString("SPECIFIC_NAME")
                .replaceAll("\\s", "")
                .matches(
                    "^FUNC112.*RETURNTABLE.*COLAVARCHAR.*,COLBNUMBER,BIN2BINARY.*,SHAREDCOLNUMBER.?$"),
            "Columns metadata SPECIFIC_NAME should contains expected columns ");
        resultSet.next();
        assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
        assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
        assertEquals("FUNC112", resultSet.getString("FUNCTION_NAME"));
        assertEquals("SHAREDCOL", resultSet.getString("COLUMN_NAME"));
        assertEquals(DatabaseMetaData.functionColumnResult, resultSet.getInt("COLUMN_TYPE"));
        assertEquals(Types.NUMERIC, resultSet.getInt("DATA_TYPE"));
        assertEquals("NUMBER", resultSet.getString("TYPE_NAME"));
        assertEquals(38, resultSet.getInt("PRECISION"));
        assertEquals(0, resultSet.getInt("LENGTH"));
        assertEquals(0, resultSet.getInt("SCALE"));
        assertEquals(10, resultSet.getInt("RADIX"));
        assertEquals(DatabaseMetaData.functionNullableUnknown, resultSet.getInt("NULLABLE"));
        assertEquals("returns table of 4 columns", resultSet.getString("REMARKS"));
        assertEquals(0, resultSet.getInt("CHAR_OCTET_LENGTH"));
        assertEquals(4, resultSet.getInt("ORDINAL_POSITION"));
        assertEquals("", resultSet.getString("IS_NULLABLE"));
        assertTrue(
            resultSet
                .getString("SPECIFIC_NAME")
                .replaceAll("\\s", "")
                .matches(
                    "^FUNC112.*RETURNTABLE.*COLAVARCHAR.*,COLBNUMBER,BIN2BINARY.*,SHAREDCOLNUMBER.?$"),
            "Columns metadata SPECIFIC_NAME should contains expected columns ");
        // setting catalog to % will result in 0 columns. % does not apply for catalog, only for
        // other
        // params
      }
      try (ResultSet resultSet = databaseMetaData.getFunctionColumns("%", "%", "%", "%")) {
        assertEquals(0, getSizeOfResultSet(resultSet));
      }
    }
  }

  /** Tests that calling getTables() concurrently doesn't cause data race condition. */
  @Test
  @SkipNewDriver("not yet implemented")
  public void testGetTablesRaceCondition()
      throws SQLException, ExecutionException, InterruptedException {
    try (Connection connection = openConnection()) {
      String database = connection.getCatalog();
      String schema = connection.getSchema();
      DatabaseMetaData databaseMetaData = connection.getMetaData();

      // Create 10 threads, each calls getTables() concurrently
      ExecutorService executorService = Executors.newFixedThreadPool(10);
      List<Future<?>> futures = new ArrayList<>();
      for (int i = 0; i < 10; i++) {
        futures.add(
            executorService.submit(
                () -> {
                  try {
                    databaseMetaData.getTables(database, schema, null, null);
                  } catch (SQLException e) {
                    throw new RuntimeException(e);
                  }
                }));
      }
      executorService.shutdown();
      for (int i = 0; i < 10; i++) {
        futures.get(i).get();
      }
    }
  }
}
