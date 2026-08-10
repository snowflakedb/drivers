package net.snowflake.client.internal.api.implementation.resultset;

/** Forward-only cursor that advances through result rows. */
interface RowCursor extends AutoCloseable {

  boolean next();

  @Override
  void close();

  boolean isClosed();

  boolean isBeforeFirst();

  boolean isAfterLast();

  boolean isFirst();

  boolean isLast();

  int getCurrentRow();
}
