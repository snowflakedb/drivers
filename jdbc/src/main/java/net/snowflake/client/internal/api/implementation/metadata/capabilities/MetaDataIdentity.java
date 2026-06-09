package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import java.sql.SQLException;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.util.NotImplementedException;

public final class MetaDataIdentity {
  private static final String DATABASE_PRODUCT_NAME = "Snowflake";
  private static final char SEARCH_STRING_ESCAPE = '\\';

  private final SnowflakeConnectionImpl connection;

  public MetaDataIdentity(SnowflakeConnectionImpl connection) {
    this.connection = connection;
  }

  public String getURL() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getUserName() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getDatabaseProductName() throws SQLException {
    connection.checkClosed();
    return DATABASE_PRODUCT_NAME;
  }

  public String getDatabaseProductVersion() throws SQLException {
    connection.checkClosed();
    return connection.getDatabaseVersion();
  }

  public String getDriverName() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.DRIVER_NAME;
  }

  public String getDriverVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.DRIVER_VERSION;
  }

  public int getDriverMajorVersion() {
    return SnowflakeDriver.MAJOR_VERSION;
  }

  public int getDriverMinorVersion() {
    return SnowflakeDriver.MINOR_VERSION;
  }

  public String getIdentifierQuoteString() throws SQLException {
    connection.checkClosed();
    return "\"";
  }

  public String getSQLKeywords() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getNumericFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getStringFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getSystemFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getTimeDateFunctions() throws SQLException {
    connection.checkClosed();
    throw new NotImplementedException();
  }

  public String getSearchStringEscape() throws SQLException {
    connection.checkClosed();
    return Character.toString(SEARCH_STRING_ESCAPE);
  }

  public String getExtraNameCharacters() throws SQLException {
    connection.checkClosed();
    return "$";
  }

  public String getSchemaTerm() throws SQLException {
    connection.checkClosed();
    return "schema";
  }

  public String getProcedureTerm() throws SQLException {
    connection.checkClosed();
    return "procedure";
  }

  public String getCatalogTerm() throws SQLException {
    connection.checkClosed();
    return "database";
  }

  public String getCatalogSeparator() throws SQLException {
    connection.checkClosed();
    return ".";
  }

  public int getDatabaseMajorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMajorVersion();
  }

  public int getDatabaseMinorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMinorVersion();
  }

  public int getJDBCMajorVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.JDBC_SPEC_MAJOR;
  }

  public int getJDBCMinorVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.JDBC_SPEC_MINOR;
  }
}
