package com.snowflake.jdbc;

import java.sql.Connection;
import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import java.util.logging.Logger;

/**
 * Snowflake JDBC Driver implementation
 *
 * <p>This is a stub implementation that provides the basic JDBC Driver interface and delegates to
 * native Rust implementation via JNI.
 */
public class SnowflakeDriver implements Driver {

  private static final String DRIVER_NAME = "Snowflake JDBC Driver";
  private static final String DRIVER_VERSION = "0.1.0";
  private static final int MAJOR_VERSION = 0;
  private static final int MINOR_VERSION = 1;

  public static void empty() {}

  public static void registerDriver() {
    try {
      DriverManager.registerDriver(new SnowflakeDriver());
    } catch (SQLException e) {
      throw new RuntimeException("Failed to register Snowflake JDBC driver", e);
    }
  }

  static {
    registerDriver();
  }

  @Override
  public Connection connect(String url, Properties info) throws SQLException {
    if (!acceptsURL(url)) {
      return null;
    }

    return new SnowflakeConnection(url, info);
  }

  @Override
  public boolean acceptsURL(String url) throws SQLException {
    if (url == null) {
      return false;
    }
    return url.startsWith("jdbc:snowflake:");
  }

  @Override
  public DriverPropertyInfo[] getPropertyInfo(String url, Properties info) throws SQLException {
    // Return empty array for stub implementation
    return new DriverPropertyInfo[0];
  }

  @Override
  public int getMajorVersion() {
    return MAJOR_VERSION;
  }

  @Override
  public int getMinorVersion() {
    return MINOR_VERSION;
  }

  @Override
  public boolean jdbcCompliant() {
    return false; // Not fully compliant in stub implementation
  }

  @Override
  public Logger getParentLogger() throws SQLFeatureNotSupportedException {
    throw new SQLFeatureNotSupportedException("getParentLogger not supported");
  }
}
