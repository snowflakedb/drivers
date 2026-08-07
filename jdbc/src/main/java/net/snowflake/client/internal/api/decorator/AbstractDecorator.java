package net.snowflake.client.internal.api.decorator;

import java.sql.SQLException;
import java.sql.Wrapper;
import java.util.function.Supplier;
import net.snowflake.client.internal.api.implementation.exception.SqlExceptionMapper;
import net.snowflake.client.internal.util.DelegatingWrapper;

/**
 * Base for every JDBC decorator: holds the delegate and {@link Telemetry} emitter and exposes the
 * shared exception-translation entry points. {@link DelegatingWrapper} resolves {@code
 * unwrap}/{@code isWrapperFor} through {@link #getDelegate()} so callers can still reach the
 * Snowflake-specific types.
 *
 * <p>Single interception point for wrapper telemetry. Curated boundaries pass an op name via the
 * instrumented {@code call/run(op, …)} overloads and record api-usage. Hot per-row / per-column
 * accessors ({@code @NoTelemetry}) instead call the delegate directly and translate via {@link
 * #translateHot}, skipping the lambda/{@code invoke} indirection entirely so the fetch loop stays
 * JIT-inlinable. A thread-local guard ensures only the <em>outermost</em> decorated call records,
 * so delegated inner calls do not double-count.
 *
 * <p>Implements {@link Wrapper}, not {@link DelegatingWrapper}: a decorator must re-expose the
 * checked {@code throws SQLException} on {@code unwrap}/{@code isWrapperFor}, so it reuses the
 * shared walking logic via {@link DelegatingWrapper#resolveUnwrap} rather than inheriting the
 * narrowed defaults.
 *
 * @param <D> the delegate type being wrapped
 */
public abstract class AbstractDecorator<D> implements Wrapper {

  /** True while a decorated call is on the stack, so only the outermost one records api-usage. */
  private static final ThreadLocal<Boolean> IN_CALL = ThreadLocal.withInitial(() -> Boolean.FALSE);

  protected final D delegate;
  protected final Telemetry telemetry;

  protected AbstractDecorator(D delegate, Telemetry telemetry) {
    this.delegate = delegate;
    // Coalesce to the NOOP sentinel so the boundary never NPEs when no emitter is wired.
    this.telemetry = telemetry == null ? Telemetry.NOOP : telemetry;
  }

  public Object getDelegate() {
    return delegate;
  }

  // The resolve* helpers throw the runtime carrier; route through call() so the boundary
  // translates it back to a checked SQLException for the public contract.
  @Override
  public <T> T unwrap(Class<T> iface) throws SQLException {
    return call(() -> DelegatingWrapper.resolveUnwrap(this, delegate, iface));
  }

  @Override
  public boolean isWrapperFor(Class<?> iface) throws SQLException {
    return call(() -> DelegatingWrapper.resolveIsWrapperFor(this, delegate, iface));
  }

  /**
   * Uninstrumented value-returning boundary (only {@code unwrap}/{@code isWrapperFor}): translate
   * only, record nothing. Hot per-row accessors bypass this and use {@link #translateHot} directly.
   */
  protected <T> T call(Supplier<T> action) throws SQLException {
    return invoke(null, action);
  }

  /** Uninstrumented void boundary: translate only, record nothing. */
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

  /**
   * Hot-path exception translation for {@link
   * net.snowflake.client.internal.codegen.NoTelemetry}-marked accessors (per-row / per-column).
   * They call the delegate directly instead of through {@link #invoke}, so they pay no per-call
   * lambda allocation, no {@link ThreadLocal} lookup on success, and don't share the megamorphic
   * {@code invoke} dispatch that defeats JIT inlining of the tight fetch loop. The {@link
   * ThreadLocal} is read only here, on the rare exception path, preserving {@code invoke}'s "only
   * the outermost call records" rule. Returns the translated checked exception for the generated
   * accessor to throw (directly or via {@code sneakyThrow}).
   */
  protected final SQLException translateHot(RuntimeException e) {
    if (!IN_CALL.get()) {
      telemetry.recordWrapperError(e);
    }
    return SqlExceptionMapper.translate(e);
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
      if (outermost) {
        telemetry.recordWrapperError(e);
      }
      throw SqlExceptionMapper.translate(e);
    } finally {
      if (outermost) {
        IN_CALL.remove();
      }
    }
  }
}
