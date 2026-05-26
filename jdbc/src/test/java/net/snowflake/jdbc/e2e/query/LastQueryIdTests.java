package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Statement;
import java.util.regex.Pattern;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSetMetaData;
import net.snowflake.client.api.statement.SnowflakeStatement;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class LastQueryIdTests extends SnowflakeIntegrationTestBase {

  private static final Pattern QUERY_ID_PATTERN =
      Pattern.compile("[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}");

  @Test
  public void shouldExposeSameLastQueryIdAcrossStatementResultSetAndMetaData() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When a SELECT is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT 1")) {
      String statementQueryId = statement.unwrap(SnowflakeStatement.class).getQueryID();
      String resultSetQueryId = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();
      ResultSetMetaData metaData = resultSet.getMetaData();
      String metaDataQueryId = metaData.unwrap(SnowflakeResultSetMetaData.class).getQueryID();

      // Then a valid Snowflake query ID is exposed
      assertNotNull(statementQueryId, "Expected a non-null query ID");
      assertTrue(
          QUERY_ID_PATTERN.matcher(statementQueryId).matches(),
          "Expected a Snowflake-shaped query ID, got: " + statementQueryId);

      // And the same ID is reachable through the Statement, the ResultSet, and the
      // ResultSetMetaData
      assertEquals(statementQueryId, resultSetQueryId);
      assertEquals(statementQueryId, metaDataQueryId);
    }
  }
}
