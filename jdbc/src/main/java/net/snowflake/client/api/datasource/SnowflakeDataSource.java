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

  void setUrl(String url);

  void setUser(String user);

  void setPassword(String password);

  void setAccount(String account);

  void setDatabase(String database);

  void setDatabaseName(String databaseName);

  void setSchema(String schema);

  void setRole(String role);

  void setWarehouse(String warehouse);

  void setPortNumber(int portNumber);

  void setServerName(String serverName);

  void setSsl(boolean ssl);

  void setPrivateKeyFile(String location, String password);

  String getUrl();

  Properties getProperties();
}
