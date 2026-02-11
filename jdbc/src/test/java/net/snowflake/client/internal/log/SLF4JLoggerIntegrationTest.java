package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Proxy;
import net.snowflake.client.internal.util.MaskedException;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;

public class SLF4JLoggerIntegrationTest {
  @Test
  public void testMaskedMessage() {
    CapturedLog capturedLog = new CapturedLog();
    Logger backend = createCapturingLogger(capturedLog);
    SFLogger logger = new SLF4JLogger(backend);

    logger.error("password=TopSecret123", true);
    assertNotNull(capturedLog.message);
    assertTrue(capturedLog.message.contains("password=****"), "masked password should be logged");
    assertFalse(capturedLog.message.contains("TopSecret123"), "raw password should not be logged");
  }

  @Test
  public void testMaskedExceptionThroughLogger() {
    CapturedLog capturedLog = new CapturedLog();
    Logger backend = createCapturingLogger(capturedLog);
    SFLogger logger = new SLF4JLogger(backend);

    RuntimeException raw = new RuntimeException("aws_secret_key='AbCdEf123'");
    logger.error("connection failed", raw);
    assertNotNull(capturedLog.throwable);
    assertInstanceOf(
        MaskedException.class,
        capturedLog.throwable,
        "throwable should be wrapped in MaskedException");
    assertNotNull(capturedLog.throwable.getMessage());
    assertTrue(
        capturedLog.throwable.getMessage().contains("aws_secret_key='****'"),
        "exception message should be masked");
    assertFalse(
        capturedLog.throwable.getMessage().contains("AbCdEf123"),
        "raw exception secret should not be present");
  }

  @Test
  public void testNullLoggerNameThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((String) null));
  }

  @Test
  public void testNullLoggerClassThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((Class<?>) null));
  }

  private static Logger createCapturingLogger(CapturedLog capturedLog) {
    InvocationHandler handler =
        (proxy, method, args) -> {
          String name = method.getName();
          if (name.startsWith("is") && name.endsWith("Enabled")) {
            return true;
          }
          switch (name) {
            case "getName":
            case "toString":
              return "capturing-logger";
            case "error":
            case "warn":
            case "info":
            case "debug":
            case "trace":
              capture(capturedLog, args);
              break;
          }
          return null;
        };

    return (Logger)
        Proxy.newProxyInstance(
            Logger.class.getClassLoader(), new Class<?>[] {Logger.class}, handler);
  }

  private static void capture(CapturedLog capturedLog, Object[] args) {
    if (args == null || args.length == 0) {
      return;
    }
    Object first = args[0];
    if (first instanceof String) {
      capturedLog.message = (String) first;
    }
    Object last = args[args.length - 1];
    if (last instanceof Throwable) {
      capturedLog.throwable = (Throwable) last;
    } else {
      capturedLog.throwable = null;
    }
  }

  private static final class CapturedLog {
    private String message;
    private Throwable throwable;
  }
}
