package net.snowflake.client.internal.api.decorator;

/**
 * Fire-and-forget wrapper-telemetry emitter for the JDBC decorator layer, emitting the two signals
 * the driver family shares: which public API was called and which errors the wrapper caught. The
 * front-end only emits; sf_core owns spans, timing, batching, and transport.
 *
 * <p>This is the stable seam: decorators carry {@link #NOOP} until the real emitter (wired from the
 * connection) drops in behind these signatures, changing no constructor or method body. The op-name
 * / error-source strings are a wire contract shared with the Python and ODBC front-ends.
 * Implementations MUST swallow every exception — telemetry must never break the user's workflow.
 */
public interface Telemetry {

  /**
   * {@code apiMethod} is the stable {@code "<JDBC interface>.<method>"} wire string (e.g. {@code
   * "Statement.execute"}). Never pass argument values.
   */
  void recordApiUsage(String apiMethod);

  /**
   * Records an error caught at a decorated boundary. The decorator hands over the raw throwable;
   * the implementation classifies it (simple class name + snake_case error-source category) so the
   * raw error never crosses the wire.
   */
  void recordWrapperError(Throwable error);

  /** No-op used where no connection is in scope, and everywhere until the real emitter is wired. */
  Telemetry NOOP =
      new Telemetry() {
        @Override
        public void recordApiUsage(String apiMethod) {}

        @Override
        public void recordWrapperError(Throwable error) {}
      };
}
