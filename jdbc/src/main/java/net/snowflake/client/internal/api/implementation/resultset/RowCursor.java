package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;

/** Forward-only cursor that advances through result rows. */
interface RowCursor extends AutoCloseable {

  boolean next() throws SQLException;

  @Override
  void close() throws SQLException;

  boolean isClosed();

  boolean isBeforeFirst();

  boolean isAfterLast();

  boolean isFirst();

  int getCurrentRow();
}
