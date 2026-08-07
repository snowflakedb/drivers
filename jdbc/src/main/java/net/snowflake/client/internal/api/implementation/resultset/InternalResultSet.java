package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.ResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSet;

public interface InternalResultSet extends ResultSet, SnowflakeResultSet {
  @Override
  boolean isClosed();

  @Override
  void close();

  @Override
  boolean next();

  @Override
  String getString(String columnLabel);
}
