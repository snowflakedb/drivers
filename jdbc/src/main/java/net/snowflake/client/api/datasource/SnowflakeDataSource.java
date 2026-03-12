package net.snowflake.client.api.datasource;

import java.util.Properties;
import javax.sql.DataSource;

/**
 * Snowflake-specific extension of {@link DataSource} that provides configuration methods for
 * Snowflake JDBC connections.
 *
 * <p>Use {@link SnowflakeDataSourceFactory} to create instances of this interface.
 */
public interface SnowflakeDataSource extends DataSource {
  // Only a minimal set of DataSource parameters has been migrated here.
  // More will be added once the parameter strategy for the core driver is finalized.

  void setUrl(String url);

  void setUser(String user);

  void setPassword(String password);

  void setServerName(String serverName);

  void setPortNumber(int portNumber);

  void setAccount(String account);

  void setDatabase(String database);

  void setSchema(String schema);

  void setRole(String role);

  void setWarehouse(String warehouse);

  String getUrl();

  Properties getProperties();
}
