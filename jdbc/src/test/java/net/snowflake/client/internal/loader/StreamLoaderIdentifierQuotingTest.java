package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.EnumMap;
import java.util.Map;
import net.snowflake.client.api.loader.LoaderProperty;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class StreamLoaderIdentifierQuotingTest {

  @ParameterizedTest
  @CsvSource(
      delimiter = '|',
      value = {
        "plain|\"plain\"",
        "with space|\"with space\"",
        "a\"b|\"a\"\"b\"",
        "\"|\"\"\"\"",
        "x\" ; DROP TABLE T; --|\"x\"\" ; DROP TABLE T; --\""
      })
  void shouldQuoteIdentifierAndDoubleEmbeddedQuotes(String name, String expected) {
    assertEquals(expected, StreamLoader.quoteIdentifier(name));
  }

  @Test
  void shouldQuoteEveryPartOfTheFullTableName() throws SQLException {
    StreamLoader loader =
        loaderWith(tableProperties("D\"B", "S\"C", "T\"B"), mock(Connection.class));

    assertEquals("\"D\"\"B\".\"S\"\"C\".\"T\"\"B\"", loader.getFullTableName());
  }

  @Test
  void shouldOmitMissingDatabaseAndSchemaFromTheFullTableName() throws SQLException {
    StreamLoader loader = loaderWith(tableProperties(null, null, "T"), mock(Connection.class));

    assertEquals("\"T\"", loader.getFullTableName());
  }

  @Test
  void shouldQuoteEveryColumnInTheColumnList() throws SQLException {
    Connection connection = mock(Connection.class);
    DatabaseMetaData metaData = mock(DatabaseMetaData.class);
    ResultSet noColumns = mock(ResultSet.class);
    when(connection.getMetaData()).thenReturn(metaData);
    when(metaData.getColumns(any(), any(), any(), any())).thenReturn(noColumns);
    Map<LoaderProperty, Object> properties = tableProperties("D", "S", "T");
    properties.put(LoaderProperty.columns, Arrays.asList("ID", "C\"1"));

    StreamLoader loader = loaderWith(properties, connection);

    assertEquals("\"ID\",\"C\"\"1\"", loader.getColumnsAsString());
  }

  private static Map<LoaderProperty, Object> tableProperties(
      String database, String schema, String table) {
    Map<LoaderProperty, Object> properties = new EnumMap<>(LoaderProperty.class);
    properties.put(LoaderProperty.databaseName, database);
    properties.put(LoaderProperty.schemaName, schema);
    properties.put(LoaderProperty.tableName, table);
    return properties;
  }

  private static StreamLoader loaderWith(Map<LoaderProperty, Object> properties, Connection c) {
    return new StreamLoader(properties, c, c);
  }
}
