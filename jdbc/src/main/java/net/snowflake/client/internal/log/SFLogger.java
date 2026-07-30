package net.snowflake.client.internal.log;

public interface SFLogger {
  boolean isDebugEnabled();

  boolean isErrorEnabled();

  boolean isInfoEnabled();

  boolean isWarnEnabled();

  void debug(String msg);

  void debug(String msg, Object... arguments);

  void debug(String msg, Throwable t);

  void error(String msg);

  void error(String msg, Object... arguments);

  void error(String msg, Throwable t);

  void info(String msg);

  void info(String msg, Object... arguments);

  void info(String msg, Throwable t);

  void warn(String msg);

  void warn(String msg, Object... arguments);

  void warn(String msg, Throwable t);
}
