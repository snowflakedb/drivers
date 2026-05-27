package net.snowflake.client.api.statement;

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
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class LastQueryIdTests extends SnowflakeIntegrationTestBase {

  private static final Pattern QUERY_ID_PATTERN =
      Pattern.compile("[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}");

  @Test
  public void shouldExposeSameLastQueryIdAcrossStatementResultSetAndMetaData() throws Exception {
    Connection connection = getDefaultConnection();

    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery("SELECT 1")) {
      String statementQueryId = statement.unwrap(SnowflakeStatement.class).getQueryID();
      String resultSetQueryId = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();
      ResultSetMetaData metaData = resultSet.getMetaData();
      String metaDataQueryId = metaData.unwrap(SnowflakeResultSetMetaData.class).getQueryID();

      assertNotNull(statementQueryId, "Expected a non-null query ID");
      assertTrue(
          QUERY_ID_PATTERN.matcher(statementQueryId).matches(),
          "Expected a Snowflake-shaped query ID, got: " + statementQueryId);

      assertEquals(statementQueryId, resultSetQueryId);
      assertEquals(statementQueryId, metaDataQueryId);
    }
  }

  @Test
  public void shouldExposeLastQueryIdOnClosedResultSet() throws Exception {
    Connection connection = getDefaultConnection();

    try (Statement statement = connection.createStatement()) {
      ResultSet resultSet = statement.executeQuery("SELECT 1");
      String queryIdBeforeClose = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();

      resultSet.close();

      String queryIdAfterClose = resultSet.unwrap(SnowflakeResultSet.class).getQueryID();
      assertEquals(queryIdBeforeClose, queryIdAfterClose);
    }
  }
}
