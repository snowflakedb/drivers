package net.snowflake.jdbc.e2e.parity;

import java.io.File;
import java.io.InputStreamReader;
import java.net.MalformedURLException;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.Driver;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * Loads two driver implementations into isolated URLClassLoaders rooted at the platform
 * classloader, so both can run side-by-side in the same JVM despite sharing the {@code
 * net.snowflake.client.*} package namespace.
 *
 * <p>Classpaths are passed via system properties:
 *
 * <ul>
 *   <li>{@code parity.newClasspath} - universal-driver runtime classpath
 *   <li>{@code parity.oldClasspath} - legacy snowflake-jdbc 4.0.1 runtime classpath
 * </ul>
 *
 * <p>On open, both connections are forced onto the Arrow result format via {@code ALTER SESSION SET
 * JDBC_QUERY_RESULT_FORMAT = 'ARROW'} so the parity matrix only exercises the Arrow path.
 */
public final class ParityHarness implements AutoCloseable {

  private static final String DRIVER_FQN = "net.snowflake.client.jdbc.SnowflakeDriver";

  private final URLClassLoader newDriverLoader;
  private final URLClassLoader oldDriverLoader;
  private final ParitySession newSession;
  private final ParitySession oldSession;

  private ParityHarness(
      URLClassLoader newDriverLoader,
      URLClassLoader oldDriverLoader,
      ParitySession newSession,
      ParitySession oldSession) {
    this.newDriverLoader = newDriverLoader;
    this.oldDriverLoader = oldDriverLoader;
    this.newSession = newSession;
    this.oldSession = oldSession;
  }

  public static ParityHarness open() throws Exception {
    URL[] newUrls = parseClasspath("parity.newClasspath");
    URL[] oldUrls = parseClasspath("parity.oldClasspath");

    // Parent = platform classloader (JDK 9+) or its JDK 8 equivalent. Critically NOT the system
    // classloader, since the test runner has universal-driver on its classpath; using it as
    // parent would let parent-first delegation steal the legacy driver's class lookups.
    ClassLoader parent = ClassLoader.getSystemClassLoader().getParent();

    URLClassLoader newCl = new URLClassLoader(newUrls, parent);
    URLClassLoader oldCl = new URLClassLoader(oldUrls, parent);

    Properties props = loadConnectionProps();
    String url = buildJdbcUrl(props);

    Connection newConn = null;
    Connection oldConn = null;
    try {
      newConn = openVia(newCl, url, props);
      oldConn = openVia(oldCl, url, props);
      ensureDatabaseAndSchema(newConn, props);
      ensureDatabaseAndSchema(oldConn, props);
      forceArrow(newConn);
      forceArrow(oldConn);
    } catch (Exception e) {
      closeQuietly(newConn);
      closeQuietly(oldConn);
      try {
        newCl.close();
      } catch (Exception ignore) {
        // ignore
      }
      try {
        oldCl.close();
      } catch (Exception ignore) {
        // ignore
      }
      throw e;
    }
    return new ParityHarness(newCl, oldCl, new ParitySession(newConn), new ParitySession(oldConn));
  }

  public ParitySession newSession() {
    return newSession;
  }

  public ParitySession oldSession() {
    return oldSession;
  }

  /** Apply the same (tz, format, overlay) on both sessions, with memoization. */
  public void applyBoth(
      String tz, String formatParam, String formatValue, java.util.Map<String, String> overlay)
      throws SQLException {
    newSession.applyWithOverlay(tz, formatParam, formatValue, overlay);
    oldSession.applyWithOverlay(tz, formatParam, formatValue, overlay);
  }

  @Override
  public void close() {
    closeQuietly(newSession.connection());
    closeQuietly(oldSession.connection());
    try {
      newDriverLoader.close();
    } catch (Exception ignore) {
      // ignore
    }
    try {
      oldDriverLoader.close();
    } catch (Exception ignore) {
      // ignore
    }
  }

  private static Connection openVia(URLClassLoader loader, String url, Properties props)
      throws Exception {
    Class<?> drvCls = Class.forName(DRIVER_FQN, true, loader);
    Driver drv = (Driver) drvCls.getDeclaredConstructor().newInstance();
    Connection conn = drv.connect(url, props);
    if (conn == null) {
      throw new IllegalStateException(
          "Driver " + drvCls + " (loader " + loader + ") refused URL " + url);
    }
    return conn;
  }

  private static URL[] parseClasspath(String sysProp) throws MalformedURLException {
    String raw = System.getProperty(sysProp);
    if (raw == null || raw.isEmpty()) {
      throw new IllegalStateException(
          "System property " + sysProp + " is not set; run via the parityTest Gradle task.");
    }
    List<URL> urls = new ArrayList<>();
    for (String entry : raw.split(File.pathSeparator)) {
      if (entry.isEmpty()) {
        continue;
      }
      urls.add(new File(entry).toURI().toURL());
    }
    return urls.toArray(new URL[0]);
  }

  private static Properties loadConnectionProps() throws Exception {
    String paramPath = System.getenv("PARAMETER_PATH");
    if (paramPath == null) {
      paramPath = "/parameters.json";
    }
    JSONObject root;
    try (InputStreamReader r = new InputStreamReader(Files.newInputStream(Paths.get(paramPath)))) {
      root = new JSONObject(new JSONTokener(r));
    }
    JSONObject params = root.getJSONObject("testconnection");

    Properties props = new Properties();
    props.setProperty("user", params.getString("SNOWFLAKE_TEST_USER"));
    props.setProperty("password", params.getString("SNOWFLAKE_TEST_PASSWORD"));
    props.setProperty("db", params.getString("SNOWFLAKE_TEST_DATABASE"));
    props.setProperty("schema", params.getString("SNOWFLAKE_TEST_SCHEMA"));
    props.setProperty(
        "warehouse",
        params.has("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            ? params.getString("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            : params.getString("SNOWFLAKE_TEST_WAREHOUSE"));
    props.setProperty("account", params.getString("SNOWFLAKE_TEST_ACCOUNT"));
    if (params.has("SNOWFLAKE_TEST_PORT")) {
      props.setProperty("port", String.valueOf(params.getInt("SNOWFLAKE_TEST_PORT")));
    }
    if (params.has("SNOWFLAKE_TEST_ROLE")) {
      props.setProperty("role", params.getString("SNOWFLAKE_TEST_ROLE"));
    }
    if (params.has("SNOWFLAKE_TEST_HOST")) {
      props.setProperty("host", params.getString("SNOWFLAKE_TEST_HOST"));
    }
    if (params.has("SNOWFLAKE_TEST_PROTOCOL")) {
      props.setProperty("protocol", params.getString("SNOWFLAKE_TEST_PROTOCOL"));
    }
    return props;
  }

  private static String buildJdbcUrl(Properties props) {
    String url = props.getProperty("url");
    if (url != null && !url.isEmpty()) {
      return url;
    }
    String built = "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
    if (props.getProperty("port") != null) {
      built += ":" + props.getProperty("port");
    }
    return built;
  }

  private static void ensureDatabaseAndSchema(Connection conn, Properties props)
      throws SQLException {
    String database = props.getProperty("db");
    String schema = props.getProperty("schema");
    try (Statement stmt = conn.createStatement()) {
      if (database != null && !database.isEmpty()) {
        stmt.execute("USE DATABASE " + database);
      }
      if (schema != null && !schema.isEmpty()) {
        stmt.execute("USE SCHEMA " + schema);
      }
    }
  }

  private static void forceArrow(Connection conn) throws SQLException {
    try (Statement stmt = conn.createStatement()) {
      stmt.execute("ALTER SESSION SET JDBC_QUERY_RESULT_FORMAT = 'ARROW'");
    }
  }

  private static void closeQuietly(Connection c) {
    if (c == null) {
      return;
    }
    try {
      c.close();
    } catch (Exception ignore) {
      // ignore
    }
  }
}
