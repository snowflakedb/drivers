package net.snowflake.client.internal.log;

/** Shared {@link SFLogger} surface; backends implement the three log entry points. */
abstract class AbstractSFLogger implements SFLogger {

  protected enum LogLevel {
    ERROR,
    WARN,
    INFO,
    DEBUG
  }

  protected abstract boolean isLevelEnabled(LogLevel level);

  protected abstract void logPlain(LogLevel level, String msg, boolean isMasked);

  protected abstract void logFormat(LogLevel level, String msg, Object... arguments);

  protected abstract void logThrowable(LogLevel level, String msg, Throwable t);

  @Override
  public final boolean isDebugEnabled() {
    return isLevelEnabled(LogLevel.DEBUG);
  }

  @Override
  public final boolean isErrorEnabled() {
    return isLevelEnabled(LogLevel.ERROR);
  }

  @Override
  public final boolean isInfoEnabled() {
    return isLevelEnabled(LogLevel.INFO);
  }

  @Override
  public final boolean isWarnEnabled() {
    return isLevelEnabled(LogLevel.WARN);
  }

  @Override
  public final void debug(String msg, boolean isMasked) {
    logPlain(LogLevel.DEBUG, msg, isMasked);
  }

  @Override
  public final void debug(String msg, Object... arguments) {
    logFormat(LogLevel.DEBUG, msg, arguments);
  }

  @Override
  public final void debug(String msg, Throwable t) {
    logThrowable(LogLevel.DEBUG, msg, t);
  }

  @Override
  public final void error(String msg, boolean isMasked) {
    logPlain(LogLevel.ERROR, msg, isMasked);
  }

  @Override
  public final void error(String msg, Object... arguments) {
    logFormat(LogLevel.ERROR, msg, arguments);
  }

  @Override
  public final void error(String msg, Throwable t) {
    logThrowable(LogLevel.ERROR, msg, t);
  }

  @Override
  public final void info(String msg, boolean isMasked) {
    logPlain(LogLevel.INFO, msg, isMasked);
  }

  @Override
  public final void info(String msg, Object... arguments) {
    logFormat(LogLevel.INFO, msg, arguments);
  }

  @Override
  public final void info(String msg, Throwable t) {
    logThrowable(LogLevel.INFO, msg, t);
  }

  @Override
  public final void warn(String msg, boolean isMasked) {
    logPlain(LogLevel.WARN, msg, isMasked);
  }

  @Override
  public final void warn(String msg, Object... arguments) {
    logFormat(LogLevel.WARN, msg, arguments);
  }

  @Override
  public final void warn(String msg, Throwable t) {
    logThrowable(LogLevel.WARN, msg, t);
  }
}
