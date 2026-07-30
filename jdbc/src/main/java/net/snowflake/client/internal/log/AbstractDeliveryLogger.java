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
      Throwable throwable = formatted.getThrowable();
      if (LogFormatter.deferThrowableDetailToDebug(level, throwable)) {
        deliver(level, LogFormatter.withTypeOnlyCause(formatted.getMessage(), throwable), null);
        if (isLevelEnabled(LogLevel.DEBUG)) {
          deliver(LogLevel.DEBUG, formatted.getMessage(), throwable);
        }
      } else {
        deliver(level, formatted.getMessage(), throwable);
      }
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
      if (LogFormatter.deferThrowableDetailToDebug(level, t)) {
        deliver(level, LogFormatter.withTypeOnlyCause(LogFormatter.mask(msg), t), null);
        if (isLevelEnabled(LogLevel.DEBUG)) {
          deliver(
              LogLevel.DEBUG, LogFormatter.mask(msg), t == null ? null : new MaskedException(t));
        }
      } else {
        deliver(level, LogFormatter.mask(msg), t == null ? null : new MaskedException(t));
      }
    } catch (Throwable ignored) {
      // Logging must never throw.
    }
  }

  protected abstract void deliver(LogLevel level, String message, Throwable throwable);
}
