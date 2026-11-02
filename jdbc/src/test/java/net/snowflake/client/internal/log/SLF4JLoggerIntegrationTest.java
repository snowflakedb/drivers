package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.io.IOException;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Supplier;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.spi.LocationAwareLogger;

public class SLF4JLoggerIntegrationTest {
  @Test
  public void testNullLoggerNameThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((String) null));
  }

  @Test
  public void testNullLoggerClassThrows() {
    assertThrows(IllegalArgumentException.class, () -> new SLF4JLogger((Class<?>) null));
  }

  @Test
  public void shouldDeliverPlainFormattedAndThrowableLogsToStandardSlf4jLogger() {
    Logger delegate = mock(Logger.class);
    when(delegate.isErrorEnabled()).thenReturn(true);
    when(delegate.isWarnEnabled()).thenReturn(true);
    when(delegate.isInfoEnabled()).thenReturn(true);
    when(delegate.isDebugEnabled()).thenReturn(true);

    SLF4JLogger logger = new SLF4JLogger(delegate);
    assertTrue(logger.isErrorEnabled());
    assertTrue(logger.isWarnEnabled());
    assertTrue(logger.isInfoEnabled());
    assertTrue(logger.isDebugEnabled());

    IOException cause = new IOException("socket closed");
    Supplier<String> lazyArg = () -> "computed";

    logger.error("plain error");
    logger.warn("plain warn");
    logger.info("plain info");
    logger.debug("plain debug");
    logger.error("formatted {} {}", "a", lazyArg);
    logger.debug("debug failure {}", 7, cause);
    logger.error("error with cause", cause);
    logger.debug("debug with cause", cause);

    verify(delegate).error("plain error");
    verify(delegate).warn("plain warn");
    verify(delegate).info("plain info");
    verify(delegate).debug("plain debug");
    verify(delegate).error("formatted a computed");
    verify(delegate).debug("debug failure 7", cause);
    verify(delegate).error("error with cause", cause);
    verify(delegate).debug("debug with cause", cause);
  }

  @Test
  public void shouldDeferThrowableDetailToDebugAtWarnAndInfoLevels() {
    Logger delegate = mock(Logger.class);
    when(delegate.isWarnEnabled()).thenReturn(true);
    when(delegate.isInfoEnabled()).thenReturn(true);
    when(delegate.isDebugEnabled()).thenReturn(true);

    SLF4JLogger logger = new SLF4JLogger(delegate);
    IOException cause = new IOException("reset by peer");

    logger.warn("connection retry", cause);
    logger.info("session notice", cause);
    logger.warn("attempt {} failed", 2, cause);

    verify(delegate).warn("connection retry: java.io.IOException");
    verify(delegate).debug("connection retry", cause);
    verify(delegate).info("session notice: java.io.IOException");
    verify(delegate).debug("session notice", cause);
    verify(delegate).warn("attempt 2 failed: java.io.IOException");
    verify(delegate).debug("attempt 2 failed", cause);

    Logger warnOnlyDelegate = mock(Logger.class);
    when(warnOnlyDelegate.isWarnEnabled()).thenReturn(true);
    when(warnOnlyDelegate.isDebugEnabled()).thenReturn(false);

    SLF4JLogger warnOnlyLogger = new SLF4JLogger(warnOnlyDelegate);
    warnOnlyLogger.warn("warn without debug", cause);

    verify(warnOnlyDelegate).warn("warn without debug: java.io.IOException");
    verify(warnOnlyDelegate, never()).debug(anyString(), any(Throwable.class));
  }

  @Test
  public void shouldRouteThroughLocationAwareLoggerWhenSupported() {
    LocationAwareLogger delegate = mock(LocationAwareLogger.class);
    when(delegate.isErrorEnabled()).thenReturn(true);
    when(delegate.isWarnEnabled()).thenReturn(true);
    when(delegate.isInfoEnabled()).thenReturn(true);
    when(delegate.isDebugEnabled()).thenReturn(true);

    SLF4JLogger logger = new SLF4JLogger(delegate);
    IllegalStateException cause = new IllegalStateException("bad state");

    logger.error("fatal", cause);
    logger.warn("careful");
    logger.info("hello {}", "world");
    logger.debug("trace");

    String fqcn = SLF4JLogger.class.getName();
    verify(delegate).log(null, fqcn, LocationAwareLogger.ERROR_INT, "fatal", null, cause);
    verify(delegate).log(null, fqcn, LocationAwareLogger.WARN_INT, "careful", null, null);
    verify(delegate).log(null, fqcn, LocationAwareLogger.INFO_INT, "hello world", null, null);
    verify(delegate).log(null, fqcn, LocationAwareLogger.DEBUG_INT, "trace", null, null);
  }

  @Test
  public void shouldSkipDeliveryAndSupplierEvaluationWhenLevelIsDisabledAndSwallowFailures() {
    Logger disabledDelegate = mock(Logger.class);
    SLF4JLogger disabledLogger = new SLF4JLogger(disabledDelegate);
    assertFalse(disabledLogger.isErrorEnabled());
    assertFalse(disabledLogger.isWarnEnabled());
    assertFalse(disabledLogger.isInfoEnabled());
    assertFalse(disabledLogger.isDebugEnabled());

    AtomicInteger supplierCalls = new AtomicInteger();
    Supplier<String> countingSupplier =
        () -> {
          supplierCalls.incrementAndGet();
          return "never";
        };

    disabledLogger.debug("skip {}", countingSupplier);
    disabledLogger.info("skip plain");
    disabledLogger.warn("skip throwable", new RuntimeException("ignored"));
    assertEquals(0, supplierCalls.get());
    verify(disabledDelegate, never()).debug(anyString());

    Logger failingDelegate = mock(Logger.class);
    when(failingDelegate.isErrorEnabled()).thenReturn(true);
    doThrow(new RuntimeException("appender failure")).when(failingDelegate).error(anyString());

    Supplier<String> throwingSupplier =
        () -> {
          throw new IllegalStateException("supplier failed");
        };
    SLF4JLogger resilientLogger = new SLF4JLogger(failingDelegate);
    assertDoesNotThrow(() -> resilientLogger.error("plain"));
    assertDoesNotThrow(() -> resilientLogger.error("format {}", "arg"));
    assertDoesNotThrow(() -> resilientLogger.error("supplier throw {}", throwingSupplier));
  }

  @Test
  public void shouldFormatStackTraceInLogFormatterHelpers() {
    assertEquals("failed", LogFormatter.withTypeOnlyCause("failed", null));
    assertEquals("failed", LogFormatter.appendThrowable("failed", null));

    IllegalArgumentException ex = new IllegalArgumentException("bad argument");
    String formatted = LogFormatter.appendThrowable("failed", ex);
    assertTrue(formatted.startsWith("failed" + System.lineSeparator()));
    assertTrue(formatted.contains("java.lang.IllegalArgumentException: bad argument"));
  }
}
