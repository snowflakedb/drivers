package net.snowflake.client.internal.unicore;

import lombok.AccessLevel;
import lombok.NoArgsConstructor;

/**
 * JNI bridge for sending wrapper (Java) log events into {@code sf_core}.
 *
 * <p>This is the outbound half of "logging via core": {@link
 * net.snowflake.client.internal.log.CoreLogger} calls {@link #logEvent} so a Java log is re-emitted
 * through core's single tracing pipeline (file, OTLP, in-band telemetry) and then handed back to
 * the originating Java logger by the bridge's {@code SFLoggerLayer}. See {@code
 * doc/logging/logging-architecture.md}.
 *
 * <p>{@link #logEvent} calls {@link NativeLibraryLoader#init()} before the first native call; a
 * genuine load failure surfaces as a {@code Throwable} the caller treats as "core unavailable" and
 * falls back to logging directly.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class CoreLoggingBridge {

  /** {@link #nativeLogEvent} return code when core accepted the event. */
  public static final int CORE_DELIVERED = 0;

  public static int logEvent(
      int level, String message, String file, int line, String function, String loggerName) {
    NativeLibraryLoader.init();
    return nativeLogEvent(level, message, file, line, function, loggerName);
  }

  /**
   * Emit a wrapper log event through core.
   *
   * @param level wire level (0=ERROR, 1=WARN, 2=INFO, 3+=DEBUG)
   * @param message the fully formatted, secret-masked message
   * @param file source file (may be empty)
   * @param line source line (0 when unknown)
   * @param function source function (may be empty)
   * @param loggerName the originating Java logger name; core routes the round-trip back to it
   * @return {@link #CORE_DELIVERED} on success, non-zero when the pipeline is not initialised yet
   */
  static native int nativeLogEvent(
      int level, String message, String file, int line, String function, String loggerName);
}
