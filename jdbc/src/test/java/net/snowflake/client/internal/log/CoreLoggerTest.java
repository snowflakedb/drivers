package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.internal.unicore.CoreLoggingBridge;
import org.junit.jupiter.api.Test;

public class CoreLoggerTest {

  private static final int WIRE_ERROR = 0;
  private static final int WIRE_WARN = 1;
  private static final int WIRE_INFO = 2;
  private static final int WIRE_DEBUG = 3;

  private static final class RecordingSink implements CoreLogger.CoreLogEventSink {
    int status;
    Error toThrow;
    final AtomicInteger calls = new AtomicInteger();
    int level;
    String message;
    String loggerName;

    RecordingSink(int status) {
      this.status = status;
    }

    @Override
    public int send(
        int level, String message, String file, int line, String function, String name) {
      calls.incrementAndGet();
      this.level = level;
      this.message = message;
      this.loggerName = name;
      if (toThrow != null) {
        throw toThrow;
      }
      return status;
    }
  }

  private static final class RecordingDelegate implements SFLogger {
    boolean enabled = true;
    Integer fallbackLevel;
    String fallbackMessage;

    @Override
    public boolean isDebugEnabled() {
      return enabled;
    }

    @Override
    public boolean isErrorEnabled() {
      return enabled;
    }

    @Override
    public boolean isInfoEnabled() {
      return enabled;
    }

    @Override
    public boolean isWarnEnabled() {
      return enabled;
    }

    @Override
    public void debug(String msg) {
      record(WIRE_DEBUG, msg);
    }

    @Override
    public void error(String msg) {
      record(WIRE_ERROR, msg);
    }

    @Override
    public void info(String msg) {
      record(WIRE_INFO, msg);
    }

    @Override
    public void warn(String msg) {
      record(WIRE_WARN, msg);
    }

    private void record(int level, String msg) {
      fallbackLevel = level;
      fallbackMessage = msg;
    }

    @Override
    public void debug(String msg, Object... arguments) {}

    @Override
    public void debug(String msg, Throwable t) {}

    @Override
    public void error(String msg, Object... arguments) {}

    @Override
    public void error(String msg, Throwable t) {}

    @Override
    public void info(String msg, Object... arguments) {}

    @Override
    public void info(String msg, Throwable t) {}

    @Override
    public void warn(String msg, Object... arguments) {}

    @Override
    public void warn(String msg, Throwable t) {}
  }

  @Test
  public void shouldDeferForeignThrowableDetailFromWarnToDebug() {
    List<Integer> levels = new ArrayList<>();
    List<String> messages = new ArrayList<>();
    CoreLogger.CoreLogEventSink sink =
        (level, message, file, line, function, name) -> {
          levels.add(level);
          messages.add(message);
          return CoreLoggingBridge.CORE_DELIVERED;
        };
    RecordingDelegate delegate = new RecordingDelegate();
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.warn("handled failure", new RuntimeException("secret=TopSecret"));

    assertEquals(2, levels.size());
    assertEquals(WIRE_WARN, (int) levels.get(0));
    assertEquals("handled failure: java.lang.RuntimeException", messages.get(0));
    assertFalse(messages.get(0).contains("TopSecret"));
    assertEquals(WIRE_DEBUG, (int) levels.get(1));
    assertTrue(messages.get(1).contains("TopSecret"));
  }

  @Test
  public void shouldSendFormattedEventToCoreWhenPipelineLive() {
    RecordingSink sink = new RecordingSink(CoreLoggingBridge.CORE_DELIVERED);
    RecordingDelegate delegate = new RecordingDelegate();
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.info("round trip {}", "payload");

    assertEquals(1, sink.calls.get());
    assertEquals(WIRE_INFO, sink.level);
    assertEquals("round trip payload", sink.message);
    assertEquals("net.snowflake.client.Foo", sink.loggerName);
    assertNull(delegate.fallbackLevel);
  }

  @Test
  public void shouldFallBackToDelegateWhenPipelineNotLive() {
    RecordingSink sink = new RecordingSink(1);
    RecordingDelegate delegate = new RecordingDelegate();
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.warn("early message");

    assertEquals(1, sink.calls.get());
    assertEquals(WIRE_WARN, delegate.fallbackLevel);
    assertEquals("early message", delegate.fallbackMessage);
  }

  @Test
  public void shouldFallBackWithoutThrowingAndLatchWhenNativeUnavailable() {
    RecordingSink sink = new RecordingSink(CoreLoggingBridge.CORE_DELIVERED);
    sink.toThrow = new UnsatisfiedLinkError("bridge not loaded");
    RecordingDelegate delegate = new RecordingDelegate();
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.error("first");
    assertEquals(WIRE_ERROR, delegate.fallbackLevel);
    assertEquals("first", delegate.fallbackMessage);

    logger.error("second");
    assertEquals(1, sink.calls.get());
    assertEquals("second", delegate.fallbackMessage);
  }

  @Test
  public void shouldNotCrossJniWhenLevelDisabled() {
    RecordingSink sink = new RecordingSink(CoreLoggingBridge.CORE_DELIVERED);
    RecordingDelegate delegate = new RecordingDelegate();
    delegate.enabled = false;
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.debug("filtered out {}", "x");

    assertEquals(0, sink.calls.get());
    assertNull(delegate.fallbackLevel);
  }

  @Test
  public void shouldBypassPreFilterWhenTroubleshootingEnabled() {
    RecordingSink sink = new RecordingSink(CoreLoggingBridge.CORE_DELIVERED);
    RecordingDelegate delegate = new RecordingDelegate();
    delegate.enabled = false;
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> true);

    logger.debug("troubleshooting captures this {}", "event");

    assertEquals(1, sink.calls.get());
    assertEquals(WIRE_DEBUG, sink.level);
    assertEquals("troubleshooting captures this event", sink.message);
  }
}
