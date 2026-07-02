package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import java.sql.SQLException;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;

public final class MetaDataLimits {
  private static final String MAX_VARCHAR_BINARY_SIZE_PARAM_NAME =
      "VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT";

  // Defaults to 16MB
  private static final int DEFAULT_MAX_LOB_SIZE = 16_777_216;

  private final SnowflakeConnectionImpl connection;
  private final CoreDriverApi coreDriverApi;

  public MetaDataLimits(SnowflakeConnectionImpl connection, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.coreDriverApi = coreDriverApi;
  }

  public int getMaxBinaryLiteralLength() throws SQLException {
    connection.checkClosed();
    // Two hex chars per binary byte, hence /2
    return getMaxCharLiteralLength() / 2;
  }

  public int getMaxCharLiteralLength() throws SQLException {
    connection.checkClosed();
    ConnectionGetParameterResponse response =
        coreDriverApi.connectionGetParameter(
            connection.getHandle(), MAX_VARCHAR_BINARY_SIZE_PARAM_NAME);
    if (response.hasValue()) {
      return Integer.parseInt(response.getValue());
    }
    return DEFAULT_MAX_LOB_SIZE;
  }

  public int getMaxColumnNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  public int getMaxColumnsInGroupBy() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInIndex() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInOrderBy() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInSelect() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInTable() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxConnections() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxCursorNameLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxIndexLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxSchemaNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  public int getMaxProcedureNameLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxCatalogNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  public int getMaxRowSize() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public boolean doesMaxRowSizeIncludeBlobs() throws SQLException {
    connection.checkClosed();
    return true;
  }

  public int getMaxStatementLength() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxStatements() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxTableNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }

  public int getMaxTablesInSelect() throws SQLException {
    connection.checkClosed();
    return 0;
  }

  public int getMaxUserNameLength() throws SQLException {
    connection.checkClosed();
    return 255;
  }
}
