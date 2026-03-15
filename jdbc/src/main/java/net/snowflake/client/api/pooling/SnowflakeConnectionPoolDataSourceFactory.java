package net.snowflake.client.api.pooling;

import net.snowflake.client.internal.api.implementation.pooling.SnowflakePooledConnectionDataSource;

public class SnowflakeConnectionPoolDataSourceFactory {

  private SnowflakeConnectionPoolDataSourceFactory() {
    throw new AssertionError("SnowflakeConnectionPoolDataSourceFactory cannot be instantiated");
  }

  public static SnowflakeConnectionPoolDataSource createConnectionPoolDataSource() {
    return new SnowflakePooledConnectionDataSource();
  }
}
