package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.CallableStatement;
import java.sql.Connection;
import java.sql.ParameterMetaData;
import java.sql.ResultSet;
import java.sql.Statement;
import java.sql.Types;
import java.util.UUID;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class CallableStatementTests extends SnowflakeIntegrationTestBase {

  private String procName;

  @BeforeAll
  void setUpStoredProcedures() throws Exception {
    procName = "add_nums_" + UUID.randomUUID().toString().replace("-", "");
    try (Statement stmt = getDefaultConnection().createStatement()) {
      stmt.execute(
          "CREATE OR REPLACE PROCEDURE "
              + procName
              + "(x DOUBLE, y DOUBLE) "
              + "RETURNS DOUBLE NOT NULL LANGUAGE JAVASCRIPT AS $$ return X + Y; $$");
    }
  }

  @AfterAll
  void tearDownStoredProcedures() throws Exception {
    try (Statement stmt = getDefaultConnection().createStatement()) {
      stmt.execute("DROP PROCEDURE IF EXISTS " + procName + "(DOUBLE, DOUBLE)");
    }
  }

  @Test
  void shouldExecuteStoredProcedureWithNoBindingParameters() throws Exception {
    // Given a stored procedure add_nums(DOUBLE, DOUBLE)
    Connection connection = getDefaultConnection();

    // When prepareCall is used with a literal argument
    try (CallableStatement cs = connection.prepareCall("CALL " + procName + "(1, 2)")) {
      // Then the statement reports no bind parameters
      ParameterMetaData metaData = cs.getParameterMetaData();
      assertEquals(0, metaData.getParameterCount());

      // And the statement executes and the result contains the sum
      try (ResultSet rs = cs.executeQuery()) {
        assertTrue(rs.next());
        assertEquals(3, rs.getDouble(1));
      }
    }
  }

  @Test
  void shouldExecuteStoredProcedureWithBindingParameters() throws Exception {
    // Given a stored procedure add_nums(DOUBLE, DOUBLE)
    Connection connection = getDefaultConnection();

    // When prepareCall is used with two ? placeholders
    try (CallableStatement cs = connection.prepareCall("CALL " + procName + "(?, ?)")) {
      // Then parameter metadata reports the two un-hinted binds (which describe
      // back as TEXT, since casts apply to result columns, not placeholders)
      assertCallParametersAreUnhintedText(cs.getParameterMetaData(), 2);

      cs.setDouble(1, 1);
      cs.setDouble(2, 2);
      try (ResultSet rs = cs.executeQuery()) {
        // And the statement executes and the result contains the sum
        assertTrue(rs.next());
        assertEquals(3, rs.getDouble(1));
      }
    }
  }

  @Test
  void shouldStripCurlyBracketEscapeSyntaxWithTwoBindingParameters() throws Exception {
    // Given a stored procedure called with JDBC escape syntax {call ...}
    Connection connection = getDefaultConnection();

    // When prepareCall is used with curly brackets and two ? placeholders
    try (CallableStatement cs = connection.prepareCall("{CALL " + procName + "(?, ?)}")) {
      // Then parameter metadata still reflects the two binds after the escape
      // syntax is stripped
      assertCallParametersAreUnhintedText(cs.getParameterMetaData(), 2);

      cs.setDouble(1, 1);
      cs.setDouble(2, 2);
      try (ResultSet rs = cs.executeQuery()) {
        // And the statement executes and the result contains the sum
        assertTrue(rs.next());
        assertEquals(3, rs.getDouble(1));
      }
    }
  }

  private static void assertCallParametersAreUnhintedText(ParameterMetaData metaData, int count)
      throws Exception {
    assertEquals(count, metaData.getParameterCount());
    for (int i = 1; i <= count; i++) {
      assertEquals(Types.VARCHAR, metaData.getParameterType(i));
      assertEquals("text", metaData.getParameterTypeName(i));
      // The describe reports CALL binds as not-nullable.
      assertEquals(ParameterMetaData.parameterNoNulls, metaData.isNullable(i));
    }
  }
}
