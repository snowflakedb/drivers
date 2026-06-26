package net.snowflake.client.api.metadata.reference;

import static net.snowflake.client.api.metadata.reference.DatabaseMetaDataIT.verifyResultSetMetaDataColumns;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/** Database Metadata IT */
public class DatabaseMetaDataInternalIT extends SnowflakeIntegrationTestBase {
  static String jdbcDb1;
  static String jdbcDb2;

  private Connection connection;
  private Statement statement;
  private DatabaseMetaData databaseMetaData;
  private ResultSet resultSet;

  @BeforeEach
  public void setUp() throws SQLException {
    try (Connection con = openConnection()) {
      initMetaData(con);
    }
  }

  static void initMetaData(Connection con) throws SQLException {
    jdbcDb1 = TestUtil.randomDatabaseName("JDBC_DB1");
    jdbcDb2 = TestUtil.randomDatabaseName("JDBC_DB2");
    try (Statement st = con.createStatement()) {
      st.execute("create or replace database " + jdbcDb1);
      st.execute("create or replace schema " + jdbcDb1 + ".JDBC_SCHEMA11");
      st.execute(
          "create or replace table "
              + jdbcDb1
              + ".JDBC_SCHEMA11.JDBC_TBL111(colA string, colB decimal, colC timestamp)");
      st.execute("create or replace schema " + jdbcDb1 + ".TEST_CTX");
      st.execute(
          "create or replace table "
              + jdbcDb1
              + ".TEST_CTX.JDBC_A (colA string, colB decimal, "
              + "colC number PRIMARY KEY);");
      st.execute(
          "create or replace table "
              + jdbcDb1
              + ".TEST_CTX.JDBC_B (colA string, colB decimal, "
              + "colC number FOREIGN KEY REFERENCES "
              + jdbcDb1
              + ".TEST_CTX.JDBC_A(colC));");
      st.execute("create or replace schema " + jdbcDb1 + ".JDBC_SCHEMA12");
      st.execute("create or replace table " + jdbcDb1 + ".JDBC_SCHEMA12.JDBC_TBL121(colA varchar)");
      st.execute(
          "create or replace table "
              + jdbcDb1
              + ".JDBC_SCHEMA12.JDBC_TBL122(colA NUMBER(20, 2) AUTOINCREMENT comment 'cmt"
              + " colA', colB NUMBER(20, 2) DEFAULT(3) NOT NULL, colC NUMBER(20,2) IDENTITY(20,"
              + " 2))");
      st.execute("create or replace database " + jdbcDb2);
      st.execute("create or replace schema " + jdbcDb2 + ".JDBC_SCHEMA21");
      st.execute("create or replace table " + jdbcDb2 + ".JDBC_SCHEMA21.JDBC_TBL211(colA string)");
      st.execute(
          "create or replace table "
              + jdbcDb2
              + ".JDBC_SCHEMA21.JDBC_BIN(bin1 binary(8388608), bin2 binary(100))");
    }
  }

  @AfterEach
  public void tearDown() throws SQLException {
    try (Connection con = openConnection()) {
      endMetaData(con);
    }
  }

  static void endMetaData(Connection con) throws SQLException {
    try (Statement st = con.createStatement()) {
      if (jdbcDb1 != null) {
        st.execute("drop database if exists " + jdbcDb1);
      }
      if (jdbcDb2 != null) {
        st.execute("drop database if exists " + jdbcDb2);
      }
    }
  }

  @Test
  @SkipNewDriver("not yet implemented")
  public void testGetFunctions() throws SQLException {
    connection = openConnection();
    statement = connection.createStatement();
    statement.execute(
        "create or replace function "
            + jdbcDb1
            + ".JDBC_SCHEMA11.JDBCFUNCTEST111 "
            + "(a number, b number) RETURNS NUMBER COMMENT='multiply numbers' as 'a*b'");
    statement.execute(
        "create or replace function "
            + jdbcDb1
            + ".JDBC_SCHEMA12.JDBCFUNCTEST121 "
            + "(a number, b number) RETURNS NUMBER COMMENT='multiply numbers' as 'a*b'");
    statement.execute(
        "create or replace function "
            + jdbcDb1
            + ".JDBC_SCHEMA12.JDBCFUNCTEST122 "
            + "(a number, b number) RETURNS NUMBER COMMENT='multiply numbers' as 'a*b'");
    statement.execute(
        "create or replace function "
            + jdbcDb2
            + ".JDBC_SCHEMA21.JDBCFUNCTEST211 "
            + "(a number, b number) RETURNS NUMBER COMMENT='multiply numbers' as 'a*b'");
    statement.execute(
        "create or replace function "
            + jdbcDb2
            + ".JDBC_SCHEMA21.JDBCFUNCTEST212 () RETURNS TABLE(colA"
            + " varchar) as 'select COLA from "
            + jdbcDb2
            + ".JDBC_SCHEMA21.JDBC_TBL211'");
    databaseMetaData = connection.getMetaData();

    resultSet = databaseMetaData.getFunctions(jdbcDb1, "JDBC_SCHEMA11", "JDBCFUNCTEST111");
    verifyResultSetMetaDataColumns(resultSet, MetaDataResultSetFormat.GET_FUNCTIONS);
    resultSet.next();
    assertEquals(jdbcDb1, resultSet.getString("FUNCTION_CAT"));
    assertEquals("JDBC_SCHEMA11", resultSet.getString("FUNCTION_SCHEM"));
    assertEquals("JDBCFUNCTEST111", resultSet.getString("FUNCTION_NAME"));
    assertEquals("multiply numbers", resultSet.getString("REMARKS"));
    assertEquals(DatabaseMetaData.functionNoTable, resultSet.getInt("FUNCTION_TYPE"));
    assertEquals("JDBCFUNCTEST111", resultSet.getString("SPECIFIC_NAME"));
    assertFalse(resultSet.next());

    resultSet = databaseMetaData.getFunctions(jdbcDb2, "JDBC_SCHEMA21", "JDBCFUNCTEST212");
    resultSet.next();
    assertEquals(DatabaseMetaData.functionReturnsTable, resultSet.getInt("FUNCTION_TYPE"));
    assertFalse(resultSet.next());

    resultSet = databaseMetaData.getFunctions(null, null, "AND");
    resultSet.next();
    assertEquals("", resultSet.getString("FUNCTION_CAT"));
    assertEquals("", resultSet.getString("FUNCTION_SCHEM"));
    assertEquals("AND", resultSet.getString("FUNCTION_NAME"));
    assertEquals(DatabaseMetaData.functionNoTable, resultSet.getInt("FUNCTION_TYPE"));
    assertEquals("AND", resultSet.getString("SPECIFIC_NAME"));
    assertFalse(resultSet.next());

    // Scope pattern searches to the test databases. The legacy null-catalog form
    // (getFunctions(null, ...)) enumerates SHOW FUNCTIONS across every database in the
    // account, which is what made this test take minutes on a shared account. Scoping to
    // jdbcDb1/jdbcDb2 exercises the same matching logic without the account-wide scan.
    resultSet = databaseMetaData.getFunctions(jdbcDb1, null, "JDBCFUNCTEST%");
    assertEquals(3, getSizeOfResultSet(resultSet));
    resultSet = databaseMetaData.getFunctions(jdbcDb2, null, "JDBCFUNCTEST%");
    assertEquals(2, getSizeOfResultSet(resultSet));
    resultSet = databaseMetaData.getFunctions(jdbcDb1, "JDBC_SCHEMA1_", "_DBCFUNCTEST%");
    assertEquals(3, getSizeOfResultSet(resultSet));

    resultSet = databaseMetaData.getFunctions("JDBC_DB3", "JDBC_SCHEMA1_", "_DBCFUNCTEST%");
    assertEquals(0, getSizeOfResultSet(resultSet));

    resultSet = databaseMetaData.getFunctions(jdbcDb1, "JDBC_SCHEMA__", "_DBCFUNCTEST%");
    assertEquals(3, getSizeOfResultSet(resultSet));
    resultSet = databaseMetaData.getFunctions(jdbcDb1, "JDBC_SCHEMA1_", "_DBCFUNCTEST11_");
    assertEquals(1, getSizeOfResultSet(resultSet));
    resultSet = databaseMetaData.getFunctions(jdbcDb1, null, "_DBCFUNCTEST11_");
    assertEquals(1, getSizeOfResultSet(resultSet));

    resultSet.close();
    resultSet.next();

    statement.close();
    connection.close();
  }

  @Test
  @SkipNewDriver("not yet implemented")
  public void testGetMetaDataUseConnectionCtx() throws SQLException {
    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {

      statement.execute("use database " + jdbcDb1);
      statement.execute("use schema JDBC_SCHEMA11");
      statement.execute("alter SESSION set CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX=true");

      DatabaseMetaData databaseMetaData = connection.getMetaData();

      try (ResultSet resultSet = databaseMetaData.getSchemas(null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }
      try (ResultSet resultSet = databaseMetaData.getTables(null, null, null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }

      statement.execute("use schema JDBC_SCHEMA12");
      try (ResultSet resultSet = databaseMetaData.getTables(null, null, null, null)) {
        assertEquals(2, getSizeOfResultSet(resultSet));
      }

      try (ResultSet resultSet = databaseMetaData.getColumns(null, null, null, null)) {
        assertEquals(4, getSizeOfResultSet(resultSet));
      }

      statement.execute("use schema TEST_CTX");
      try (ResultSet resultSet = databaseMetaData.getPrimaryKeys(null, null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }

      try (ResultSet resultSet = databaseMetaData.getImportedKeys(null, null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }

      try (ResultSet resultSet = databaseMetaData.getExportedKeys(null, null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }

      try (ResultSet resultSet =
          databaseMetaData.getCrossReference(null, null, null, null, null, null)) {
        assertEquals(1, getSizeOfResultSet(resultSet));
      }
    }
  }
}
