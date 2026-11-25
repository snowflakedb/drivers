package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
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
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class StreamLoaderStageColumnsTest {

  private StreamLoader loader;

  @BeforeEach
  void setUp() throws SQLException {
    ResultSet vectorColumn = mock(ResultSet.class);
    when(vectorColumn.next()).thenReturn(true);
    when(vectorColumn.getString(6)).thenReturn("VECTOR");
    when(vectorColumn.getInt(7)).thenReturn(3);
    ResultSet intColumn = mock(ResultSet.class);
    when(intColumn.next()).thenReturn(true);
    when(intColumn.getString(6)).thenReturn("NUMBER");
    DatabaseMetaData metaData = mock(DatabaseMetaData.class);
    when(metaData.getColumns(any(), any(), any(), eq("VEC"))).thenReturn(vectorColumn);
    when(metaData.getColumns(any(), any(), any(), eq("ID"))).thenReturn(intColumn);
    Connection connection = mock(Connection.class);
    when(connection.getMetaData()).thenReturn(metaData);

    Map<LoaderProperty, Object> properties = new EnumMap<>(LoaderProperty.class);
    properties.put(LoaderProperty.tableName, "T");
    properties.put(LoaderProperty.columns, Arrays.asList("ID", "VEC"));
    loader = new StreamLoader(properties, connection, connection);
  }

  @ParameterizedTest
  @ValueSource(strings = {"INT", "FLOAT", "float"})
  void shouldCastVectorColumnsToConfiguredType(String vectorType) {
    loader.setVectorColumnType(vectorType);

    assertEquals(
        "\"ID\", \"VEC\"::VECTOR(" + vectorType + ", 3)", loader.getStageColumnsAsString());
  }

  @Test
  void shouldRejectVectorTypeOutsideIntAndFloat() {
    loader.setVectorColumnType("FLOAT, 3) FROM T; DROP TABLE T; --");

    IllegalArgumentException error =
        assertThrows(IllegalArgumentException.class, loader::getStageColumnsAsString);

    assertEquals(
        "Target table with vector columns must use setVectorColumnType with \"INT\" or \"FLOAT\"",
        error.getMessage());
  }

  @Test
  void shouldRequireVectorTypeWhenTargetHasVectorColumns() {
    assertThrows(IllegalArgumentException.class, loader::getStageColumnsAsString);
  }
}
