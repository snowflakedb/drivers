package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
    Boolean fallbackMasked;

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
    public void debug(String msg, boolean isMasked) {
      record(WIRE_DEBUG, msg, isMasked);
    }

    @Override
    public void error(String msg, boolean isMasked) {
      record(WIRE_ERROR, msg, isMasked);
    }

    @Override
    public void info(String msg, boolean isMasked) {
      record(WIRE_INFO, msg, isMasked);
    }

    @Override
    public void warn(String msg, boolean isMasked) {
      record(WIRE_WARN, msg, isMasked);
    }

    private void record(int level, String msg, boolean isMasked) {
      fallbackLevel = level;
      fallbackMessage = msg;
      fallbackMasked = isMasked;
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
    assertFalse(delegate.fallbackMasked);
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
  public void shouldMaskSecretsBeforeSendingToCore() {
    RecordingSink sink = new RecordingSink(CoreLoggingBridge.CORE_DELIVERED);
    RecordingDelegate delegate = new RecordingDelegate();
    CoreLogger logger = new CoreLogger("net.snowflake.client.Foo", delegate, sink, () -> false);

    logger.error("password=TopSecret123", true);

    assertTrue(sink.message.contains("password=****"));
    assertFalse(sink.message.contains("TopSecret123"));
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
