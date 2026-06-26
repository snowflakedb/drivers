package net.snowflake.client.api.metadata.reference;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

public class DatabaseMetaDataResultSetLatestIT extends SnowflakeIntegrationTestBase {

  private Connection getConnectionWithWildcardsDisabled() throws SQLException {
    Properties props = new Properties();
    props.put("ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS", "false");
    return openConnection(props);
  }

  /** Added in > 3.17.0 */
  @Test
  @SkipNewDriver("not yet implemented")
  public void testObjectColumn() throws SQLException {
    try (Connection connection = getConnectionWithWildcardsDisabled();
        Statement statement = connection.createStatement()) {
      statement.execute("ALTER SESSION SET ENABLE_STRUCTURED_TYPES_IN_FDN_TABLES = TRUE");
      statement.execute(
          "CREATE OR REPLACE TABLE TABLEWITHOBJECTCOLUMN ("
              + "    col OBJECT("
              + "      str VARCHAR,"
              + "      num NUMBER(38,0)"
              + "      )"
              + "   )");
      DatabaseMetaData metaData = connection.getMetaData();
      try (ResultSet resultSet =
          metaData.getColumns(
              connection.getCatalog(), connection.getSchema(), "TABLEWITHOBJECTCOLUMN", null)) {
        assertTrue(resultSet.next());
        assertEquals("OBJECT", resultSet.getObject("TYPE_NAME"));
        assertFalse(resultSet.next());
      }
    }
  }
}
