package net.snowflake.client.api.driver;

import java.sql.Connection;
import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.util.Properties;
import java.util.logging.Logger;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.Decorators;
import net.snowflake.client.internal.api.implementation.connection.ConnectionString;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.exception.SqlExceptionMapper;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Snowflake JDBC Driver implementation
 *
 * <p>This is a stub implementation that provides the basic JDBC Driver interface and delegates to
 * native Rust implementation via JNI.
 */
public class SnowflakeDriver implements Driver {
  private static final SFLogger logger = SFLoggerFactory.getLogger(SnowflakeDriver.class);

  // Up to 9 digits keeps the result within Integer.MAX_VALUE so parseInt cannot overflow.
  // Declared before the constants below because their initializers call parseVersionComponent.
  private static final Pattern LEADING_DIGITS = Pattern.compile("\\d{1,9}");

  public static final String DRIVER_NAME = "Snowflake JDBC Driver";

  public static final String JDBC_SPEC_VERSION = "4.2";

  /** Sourced from {@code build.gradle}'s {@code project.version} via the generated class. */
  public static final String DRIVER_VERSION = DriverVersion.VALUE;

  // Version reported to GS at login only, decoupled from the 0.0.1 artifact version: GS's
  // validateClientVersion rejects CLIENT_APP_ID="JDBC" below its floors (min 2.3.1, crypto floors
  // up to 2.5.0) as CLIENT_TOO_OLD. DRIVER_VERSION / DatabaseMetaData stay 0.0.1.
  // Pre-release suffix is parsed by sf_core: CLIENT_APP_VERSION is stripped to "5.0.0" and
  // CLIENT_ENVIRONMENT.RELEASE_TYPE becomes "prpr3".
  public static final String CLIENT_APP_VERSION = "5.0.0.prpr3";

  public static final int MAJOR_VERSION = parseVersionComponent(DRIVER_VERSION, 0);
  public static final int MINOR_VERSION = parseVersionComponent(DRIVER_VERSION, 1);

  public static final int JDBC_SPEC_MAJOR = parseVersionComponent(JDBC_SPEC_VERSION, 0);
  public static final int JDBC_SPEC_MINOR = parseVersionComponent(JDBC_SPEC_VERSION, 1);

  public static String getDriverVersion() {
    return DRIVER_VERSION;
  }

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
    if (ConnectionString.hasUnsupportedPrefix(url)) {
      logger.debug("Connect strings must start with jdbc:snowflake://");
      return null;
    }
    ConnectionString parsed = ConnectionString.parse(url, info);
    if (!parsed.isValid()) {
      throw new SnowflakeSQLException("Connection string is invalid. Unable to parse.");
    }
    // The connection constructor performs login and throws unchecked driver carriers on failure.
    // Because a constructor runs before any instance exists, the @JdbcBoundary decorator cannot
    // wrap
    // it — so translate here to honor connect()'s throws SQLException contract instead of letting a
    // carrier (e.g. CoreException on a bad login) escape the public JDBC entry point.
    SnowflakeConnectionImpl connection =
        SqlExceptionMapper.call(() -> new SnowflakeConnectionImpl(url, info));
    return Decorators.connection(connection, connection.getTelemetry());
  }

  @Override
  public boolean acceptsURL(String url) throws SQLException {
    if (url == null) {
      throw new SQLException("URL must not be null");
    }
    return url.startsWith("jdbc:snowflake:");
  }

  @Override
  public DriverPropertyInfo[] getPropertyInfo(String url, Properties info) throws SQLException {
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
    // Not fully compliant with the JDBC 4.2 specification.
    return false;
  }

  @Override
  public Logger getParentLogger() {
    return null;
  }

  /**
   * Returns the {@code index}-th dot-separated component as a non-negative int, or {@code 0} when
   * absent or non-numeric (e.g. {@code "0-SNAPSHOT"} yields {@code 0}).
   */
  public static int parseVersionComponent(String version, int index) {
    if (version == null || index < 0) {
      return 0;
    }
    String[] parts = version.split("\\.", -1);
    if (index >= parts.length) {
      return 0;
    }
    Matcher matcher = LEADING_DIGITS.matcher(parts[index]);
    return matcher.lookingAt() ? Integer.parseInt(matcher.group()) : 0;
  }
}
