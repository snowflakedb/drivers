package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;

@RequiredArgsConstructor
public final class MetaDataLimits {

  private final InternalSnowflakeConnection connection;

  public int getMaxBinaryLiteralLength() {
    connection.checkClosed();
    // Two hex chars per binary byte, hence /2
    return getMaxCharLiteralLength() / 2;
  }

  public int getMaxCharLiteralLength() {
    connection.checkClosed();
    return connection.getParameters().getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT);
  }

  public int getMaxColumnNameLength() {
    connection.checkClosed();
    return 255;
  }

  public int getMaxColumnsInGroupBy() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInIndex() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInOrderBy() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInSelect() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxColumnsInTable() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxConnections() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxCursorNameLength() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxIndexLength() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxSchemaNameLength() {
    connection.checkClosed();
    return 255;
  }

  public int getMaxProcedureNameLength() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxCatalogNameLength() {
    connection.checkClosed();
    return 255;
  }

  public int getMaxRowSize() {
    connection.checkClosed();
    return 0;
  }

  public boolean doesMaxRowSizeIncludeBlobs() {
    connection.checkClosed();
    return true;
  }

  public int getMaxStatementLength() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxStatements() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxTableNameLength() {
    connection.checkClosed();
    return 255;
  }

  public int getMaxTablesInSelect() {
    connection.checkClosed();
    return 0;
  }

  public int getMaxUserNameLength() {
    connection.checkClosed();
    return 255;
  }
}
