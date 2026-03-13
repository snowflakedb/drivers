package net.snowflake.client.internal.api.implementation.datasource;

import java.io.PrintWriter;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import java.util.function.Supplier;
import java.util.logging.Logger;
import net.snowflake.client.api.datasource.SnowflakeDataSource;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Basic implementation of {@link SnowflakeDataSource} for Snowflake JDBC connections.
 *
 * <p>This class provides a simple, non-pooled DataSource implementation that creates new Snowflake
 * connections on demand. It is suitable for applications that do not require connection pooling or
 * for use with external connection pool managers.
 *
 * <p><b>Note:</b> This class is not intended for direct instantiation. Use {@link
 * net.snowflake.client.api.datasource.SnowflakeDataSourceFactory#createDataSource()} instead.
 */
public class SnowflakeBasicDataSource implements SnowflakeDataSource {

  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeBasicDataSource.class);

  static {
    try {
      Class.forName("net.snowflake.client.api.driver.SnowflakeDriver");
    } catch (ClassNotFoundException e) {
      throw new IllegalStateException(
          "Unable to load "
              + "net.snowflake.client.api.driver.SnowflakeDriver. "
              + "Please check if you have proper Snowflake JDBC "
              + "Driver jar on the classpath",
          e);
    }
  }

  private final Properties properties = new Properties();
  private String url;
  private String user;
  private String password;

  // DataSource methods ----------------------------------------------------------------------------

  @Override
  public Connection getConnection() throws SQLException {
    return getConnection(user, password);
  }

  @Override
  public Connection getConnection(String username, String password) throws SQLException {
    String effectiveUser = username != null ? username : user;
    try {
      Properties properties = getProperties();
      if (username != null) {
        properties.setProperty(SnowflakeSessionProperty.USER.getPropertyKey(), username);
      }
      if (password != null) {
        properties.setProperty(SnowflakeSessionProperty.PASSWORD.getPropertyKey(), password);
      }

      Connection con = DriverManager.getConnection(getUrl(), properties);
      logger.trace(
          "Created a connection for {} at {}", effectiveUser, (Supplier<String>) this::getUrl);
      return con;
    } catch (SQLException e) {
      logger.error("Failed to create a connection for {} at {}: {}", effectiveUser, getUrl(), e);
      throw e;
    }
  }

  // CommonDataSource methods ----------------------------------------------------------------------

  @Override
  public PrintWriter getLogWriter() throws SQLException {
    throw new SQLFeatureNotSupportedException();
  }

  @Override
  public void setLogWriter(PrintWriter out) throws SQLException {
    throw new SQLFeatureNotSupportedException();
  }

  @Override
  public int getLoginTimeout() {
    try {
      return Integer.parseInt(
          properties.getProperty(SnowflakeSessionProperty.LOGIN_TIMEOUT.getPropertyKey()));
    } catch (NumberFormatException e) {
      logger.warn(
          "Could not parse loginTimeout property value '{}', returning default of 0",
          properties.getProperty(SnowflakeSessionProperty.LOGIN_TIMEOUT.getPropertyKey()));
      return 0;
    }
  }

  @Override
  public void setLoginTimeout(int seconds) {
    properties.put(
        SnowflakeSessionProperty.LOGIN_TIMEOUT.getPropertyKey(), Integer.toString(seconds));
  }

  @Override
  public Logger getParentLogger() throws SQLFeatureNotSupportedException {
    throw new SQLFeatureNotSupportedException();
  }

  // Wrapper methods -------------------------------------------------------------------------------

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLFeatureNotSupportedException {
    throw new SQLFeatureNotSupportedException();
  }

  @Override
  public <T> T unwrap(Class<T> iface) throws SQLFeatureNotSupportedException {
    throw new SQLFeatureNotSupportedException();
  }

  // SnowflakeDataSource methods -------------------------------------------------------------------

  @Override
  public void setUrl(String url) {
    this.url = url;
  }

  @Override
  public void setUser(String user) {
    this.user = user;
  }

  @Override
  public void setPassword(String password) {
    this.password = password;
  }

  @Override
  public void setAccount(String account) {
    this.properties.setProperty(SnowflakeSessionProperty.ACCOUNT.getPropertyKey(), account);
  }

  @Override
  public void setDatabase(String database) {
    this.properties.setProperty(SnowflakeSessionProperty.DATABASE.getPropertyKey(), database);
  }

  @Override
  public void setSchema(String schema) {
    this.properties.setProperty(SnowflakeSessionProperty.SCHEMA.getPropertyKey(), schema);
  }

  @Override
  public void setRole(String role) {
    this.properties.setProperty(SnowflakeSessionProperty.ROLE.getPropertyKey(), role);
  }

  @Override
  public void setWarehouse(String warehouse) {
    this.properties.setProperty(SnowflakeSessionProperty.WAREHOUSE.getPropertyKey(), warehouse);
  }

  @Override
  public String getUrl() {
    return url;
  }

  @Override
  public Properties getProperties() {
    // returns the copy to avoid access to a shared mutable field
    Properties properties = new Properties();
    properties.putAll(this.properties);
    return properties;
  }
}
