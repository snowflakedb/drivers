package net.snowflake.client.internal.api.decorator;

import java.sql.SQLException;
import java.util.function.Supplier;
import net.snowflake.client.internal.util.DelegatingWrapper;

/**
 * Base for every JDBC decorator: holds the delegate and {@link Telemetry} emitter and exposes the
 * shared exception-translation entry points. A decorator delegates + translates and nothing else;
 * {@link DelegatingWrapper} resolves {@code unwrap}/{@code isWrapperFor} through {@link
 * #getDelegate()} so callers can still reach the Snowflake-specific types.
 *
 * <p>This is the single interception point for wrapper telemetry. Curated boundaries pass an op
 * name via the instrumented {@code call/run(op, …)} overloads and record api-usage; hot per-row /
 * per-column accessors use the plain overloads and record nothing. A thread-local guard ensures
 * only the <em>outermost</em> decorated call records, so delegated inner calls do not double-count.
 * Wired to {@link Telemetry#NOOP} until the real emitter lands behind these signatures.
 *
 * @param <D> the delegate type being wrapped
 */
abstract class AbstractDecorator<D> implements DelegatingWrapper {

  /** True while a decorated call is on the stack, so only the outermost one records api-usage. */
  private static final ThreadLocal<Boolean> IN_CALL = ThreadLocal.withInitial(() -> Boolean.FALSE);

  protected final D delegate;
  protected final Telemetry telemetry;

  protected AbstractDecorator(D delegate, Telemetry telemetry) {
    this.delegate = delegate;
    this.telemetry = telemetry;
  }

  @Override
  public Object getDelegate() {
    return delegate;
  }

  /** Hot value-returning accessor (e.g. {@code getString}): translate only, record nothing. */
  protected <T> T call(Supplier<T> action) throws SQLException {
    return invoke(null, action);
  }

  /** Hot void accessor: translate only, record nothing. */
  protected void run(Runnable action) throws SQLException {
    invoke(
        null,
        () -> {
          action.run();
          return null;
        });
  }

  /**
   * Curated value-returning boundary (e.g. {@code executeQuery}): translate and record {@code
   * apiMethod}.
   */
  protected <T> T call(String apiMethod, Supplier<T> action) throws SQLException {
    return invoke(apiMethod, action);
  }

  /** Curated void boundary (e.g. {@code commit}): translate and record {@code apiMethod}. */
  protected void run(String apiMethod, Runnable action) throws SQLException {
    invoke(
        apiMethod,
        () -> {
          action.run();
          return null;
        });
  }

  private <T> T invoke(String apiMethod, Supplier<T> action) throws SQLException {
    boolean outermost = !IN_CALL.get();
    if (outermost) {
      IN_CALL.set(Boolean.TRUE);
      if (apiMethod != null) {
        telemetry.recordApiUsage(apiMethod);
      }
    }
    try {
      return action.get();
    } catch (RuntimeException e) {
      // The real emitter will call telemetry.recordWrapperError(classify(e)) here.
      throw SqlExceptionMapper.translate(e);
    } finally {
      if (outermost) {
        IN_CALL.remove();
      }
    }
  }
}
