package net.snowflake.client.internal.log;

import java.time.Instant;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.util.logging.Formatter;
import java.util.logging.LogRecord;

/** Snowflake log formatter for {@link java.util.logging} file and console handlers. */
public class SFFormatter extends Formatter {
  private static final DateTimeFormatter DF =
      DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss.SSS").withZone(ZoneOffset.UTC);

  // Fixed to "net.snowflake.client" since SnowflakeDriver moved to api.driver package.
  public static final String CLASS_NAME_PREFIX = "net.snowflake.client";

  public static final String INFORMATICA_V1_CLASS_NAME_PREFIX = "com.snowflake";

  @Override
  public String format(LogRecord record) {
    int lineNumber = -1;
    String className = record.getSourceClassName();
    final String methodName = record.getSourceMethodName();
    StackTraceElement[] stackTraces = Thread.currentThread().getStackTrace();
    for (StackTraceElement ste : stackTraces) {
      if (className.equals(ste.getClassName()) && methodName.equals(ste.getMethodName())) {
        lineNumber = ste.getLineNumber();
        break;
      }
    }
    if (className.startsWith(CLASS_NAME_PREFIX)) {
      className = "n.s.c" + className.substring(CLASS_NAME_PREFIX.length());
    } else if (className.startsWith(INFORMATICA_V1_CLASS_NAME_PREFIX)) {
      className = "c.s" + className.substring(INFORMATICA_V1_CLASS_NAME_PREFIX.length());
    }

    StringBuilder builder = new StringBuilder(1000);
    builder.append(DF.format(Instant.ofEpochMilli(record.getMillis()))).append(" ");
    builder.append(className).append(" ");
    builder.append(record.getLevel()).append(" ");
    builder.append(methodName).append(":");
    builder.append(lineNumber).append(" - ");
    builder.append(formatMessage(record));
    builder.append("\n");
    return builder.toString();
  }
}
