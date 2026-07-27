package net.snowflake.client.internal.log;

import java.io.IOException;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.logging.Level;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.ConnectionString;

/**
 * Wires {@link JDK14Logger#instantiateLogger} from connection properties (legacy driver parity).
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class Jdk14LoggerBootstrap {

  private static final String TRACING_PROPERTY = "TRACING";
  private static final String DEFAULT_LOG_PATTERN = "%h/snowflake_jdbc%u.log";

  /**
   * Activates JUL file logging when {@code TRACING} is set and no external {@code
   * java.util.logging.config.file} is in use. Client-config log path/level is applied when that
   * support is ported.
   */
  public static void initFromConnectionIfConfigured(String url, Properties info)
      throws IOException {
    if (!"JUL".equals(SFLoggerFactory.getLoggerImplementationName())) {
      return;
    }
    if (System.getProperty("java.util.logging.config.file") != null) {
      return;
    }

    Map<String, Object> parameters = ConnectionString.parse(url, info).getParameters();
    Object tracing = parameters.get(TRACING_PROPERTY);
    if (tracing == null) {
      return;
    }

    Level logLevel = Level.parse(tracing.toString().toUpperCase(Locale.US));
    JDK14Logger.instantiateLogger(logLevel, DEFAULT_LOG_PATTERN);
  }
}
