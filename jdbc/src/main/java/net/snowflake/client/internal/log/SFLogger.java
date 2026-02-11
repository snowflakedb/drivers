package net.snowflake.client.internal.log;

public interface SFLogger {
  boolean isDebugEnabled();

  boolean isErrorEnabled();

  boolean isInfoEnabled();

  boolean isTraceEnabled();

  boolean isWarnEnabled();

  void debug(String msg, boolean isMasked);

  void debug(String msg, Object... arguments);

  void debug(String msg, Throwable t);

  void error(String msg, boolean isMasked);

  void error(String msg, Object... arguments);

  void error(String msg, Throwable t);

  void info(String msg, boolean isMasked);

  void info(String msg, Object... arguments);

  void info(String msg, Throwable t);

  void trace(String msg, boolean isMasked);

  void trace(String msg, Object... arguments);

  void trace(String msg, Throwable t);

  void warn(String msg, boolean isMasked);

  void warn(String msg, Object... arguments);

  void warn(String msg, Throwable t);
}
