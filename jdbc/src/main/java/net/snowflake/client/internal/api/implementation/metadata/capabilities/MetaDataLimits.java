package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;

@RequiredArgsConstructor
public final class MetaDataLimits {

  private final InternalSnowflakeConnection connection;

  public int getMaxBinaryLiteralLength() throws SQLException {
    connection.checkClosed();
    // Two hex chars per binary byte, hence /2
    return getMaxCharLiteralLength() / 2;
  }

  public int getMaxCharLiteralLength() throws SQLException {
    connection.checkClosed();
    return connection.getParameters().getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT);
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
