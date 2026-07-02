package net.snowflake.jdbc.e2e.parity;

import java.io.File;
import java.net.MalformedURLException;
import java.net.URL;
import java.net.URLClassLoader;
import java.sql.Connection;
import java.sql.Driver;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import java.util.TreeMap;
import net.snowflake.jdbc.utils.TestParameters;

/**
 * Loads two driver implementations into isolated URLClassLoaders rooted at the platform
 * classloader, so both can run side-by-side in the same JVM despite sharing the {@code
 * net.snowflake.client.*} package namespace.
 *
 * <p>Classpaths are passed via system properties:
 *
 * <ul>
 *   <li>{@code parity.newClasspath} - universal-driver runtime classpath
 *   <li>{@code parity.oldClasspath} - legacy snowflake-jdbc 4.3.1 runtime classpath
 * </ul>
 *
 * <p>On open, both connections are forced onto the Arrow result format via {@code ALTER SESSION SET
 * JDBC_QUERY_RESULT_FORMAT = 'ARROW'} so the parity matrix only exercises the Arrow path.
 *
 * <p>Most date/time session params can be flipped at runtime via {@code ALTER SESSION} (see {@link
 * Profile}), so a single long-lived connection pair suffices. A handful, however, are read only
 * from the JDBC {@link Properties} bag at connect time and ignored thereafter (e.g. {@code
 * JDBC_GET_DATE_USE_NULL_TIMEZONE}). To exercise those, {@link #sessionsFor(Map)} opens an
 * additional connection pair with the extra properties baked in, memoized by the (sorted)
 * connect-prop map so each distinct variant is opened at most once.
 */
public final class ParityHarness implements AutoCloseable {

  private static final String DRIVER_FQN = "net.snowflake.client.jdbc.SnowflakeDriver";

  private final URLClassLoader newDriverLoader;
  private final URLClassLoader oldDriverLoader;
  private final String url;
  private final Properties baseProps;
  private final SessionPair defaultPair;

  /** Memoized connect-time variants. Keyed by the sorted extra-property map. */
  private final Map<TreeMap<String, String>, SessionPair> connectVariants = new LinkedHashMap<>();

  private ParityHarness(
      URLClassLoader newDriverLoader,
      URLClassLoader oldDriverLoader,
      String url,
      Properties baseProps,
      SessionPair defaultPair) {
    this.newDriverLoader = newDriverLoader;
    this.oldDriverLoader = oldDriverLoader;
    this.url = url;
    this.baseProps = baseProps;
    this.defaultPair = defaultPair;
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
    String url = TestParameters.buildJdbcUrl(props);

    try {
      SessionPair defaultPair = openPair(newCl, oldCl, url, props);
      return new ParityHarness(newCl, oldCl, url, props, defaultPair);
    } catch (Exception e) {
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
  }

  /**
   * Return a connection pair whose underlying JDBC connections were opened with the given extra
   * properties set at connect time. Use this for params the drivers read only from the {@link
   * Properties} bag and never refresh from server responses (so {@code ALTER SESSION} cannot vary
   * them). An empty map returns the shared default pair; otherwise the pair is opened once and
   * memoized, so repeated calls with the same map reuse the same connections.
   */
  public SessionPair sessionsFor(Map<String, String> connectProps) throws Exception {
    if (connectProps == null || connectProps.isEmpty()) {
      return defaultPair;
    }
    TreeMap<String, String> key = new TreeMap<>(connectProps);
    SessionPair cached = connectVariants.get(key);
    if (cached != null) {
      return cached;
    }
    Properties props = new Properties();
    props.putAll(baseProps);
    for (Map.Entry<String, String> e : key.entrySet()) {
      props.setProperty(e.getKey(), e.getValue());
    }
    SessionPair pair = openPair(newDriverLoader, oldDriverLoader, url, props);
    connectVariants.put(key, pair);
    return pair;
  }

  @Override
  public void close() {
    defaultPair.close();
    for (SessionPair pair : connectVariants.values()) {
      pair.close();
    }
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

  /** Open one (new, old) connection pair against the same URL/props and ready it for parity use. */
  private static SessionPair openPair(
      URLClassLoader newCl, URLClassLoader oldCl, String url, Properties props) throws Exception {
    Connection newConn = null;
    Connection oldConn = null;
    try {
      newConn = openVia(newCl, url, props);
      oldConn = openVia(oldCl, url, props);
      ensureDatabaseAndSchema(newConn, props);
      ensureDatabaseAndSchema(oldConn, props);
      forceArrow(newConn);
      forceArrow(oldConn);
      return new SessionPair(new ParitySession(newConn), new ParitySession(oldConn));
    } catch (Exception e) {
      closeQuietly(newConn);
      closeQuietly(oldConn);
      throw e;
    }
  }

  /**
   * A new-driver/old-driver connection pair sharing the same connect-time properties. Each side
   * memoizes its own last-applied session state (see {@link ParitySession}).
   */
  public static final class SessionPair {
    private final ParitySession newSession;
    private final ParitySession oldSession;

    SessionPair(ParitySession newSession, ParitySession oldSession) {
      this.newSession = newSession;
      this.oldSession = oldSession;
    }

    public ParitySession newSession() {
      return newSession;
    }

    public ParitySession oldSession() {
      return oldSession;
    }

    /** Apply the same (tz, format, overlay) on both sides, with per-session memoization. */
    public void applyBoth(
        String tz, String formatParam, String formatValue, Map<String, String> overlay)
        throws SQLException {
      newSession.applyWithOverlay(tz, formatParam, formatValue, overlay);
      oldSession.applyWithOverlay(tz, formatParam, formatValue, overlay);
    }

    void close() {
      closeQuietly(newSession.connection());
      closeQuietly(oldSession.connection());
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

  private static Properties loadConnectionProps() {
    return TestParameters.withDefaultAuth(TestParameters.loadDefaultConnectionProperties());
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
