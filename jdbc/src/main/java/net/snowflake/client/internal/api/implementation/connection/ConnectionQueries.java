package net.snowflake.client.internal.api.implementation.connection;

/** SQL statements issued by {@link SnowflakeConnectionImpl}. */
final class ConnectionQueries {

  private ConnectionQueries() {}

  static final String CURRENT_VERSION = "SELECT CURRENT_VERSION()";
}
