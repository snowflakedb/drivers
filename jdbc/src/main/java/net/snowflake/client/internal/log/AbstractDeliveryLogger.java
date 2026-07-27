package net.snowflake.client.internal.log;

import net.snowflake.client.internal.util.MaskedException;

/** Direct delivery backend with shared formatting, masking, and never-throw guard. */
abstract class AbstractDeliveryLogger extends AbstractSFLogger {

  @Override
  protected final void logPlain(LogLevel level, String msg, boolean isMasked) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      deliver(level, isMasked ? LogFormatter.mask(msg) : msg, null);
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  @Override
  protected final void logFormat(LogLevel level, String msg, Object... arguments) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      LogFormatter.Formatted formatted = LogFormatter.format(msg, arguments);
      deliver(level, formatted.getMessage(), formatted.getThrowable());
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  @Override
  protected final void logThrowable(LogLevel level, String msg, Throwable t) {
    try {
      if (!isLevelEnabled(level)) {
        return;
      }
      deliver(level, LogFormatter.mask(msg), t == null ? null : new MaskedException(t));
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  protected abstract void deliver(LogLevel level, String message, Throwable throwable);
}
