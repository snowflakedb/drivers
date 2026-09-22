package net.snowflake.client.api.driver;

import java.util.Properties;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;

/** Creates the internal connection used by the public JDBC driver boundary. */
@FunctionalInterface
interface ConnectionFactory {
  SnowflakeConnectionImpl create(String url, Properties properties);
}
