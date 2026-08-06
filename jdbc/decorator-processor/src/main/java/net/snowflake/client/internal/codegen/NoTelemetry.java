package net.snowflake.client.internal.codegen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a method on a {@link JdbcBoundary}-annotated class as a hot-path accessor (per-row /
 * per-column) that should skip telemetry recording in the generated decorator. The processor then
 * emits the un-instrumented {@code call(() -> …)} / {@code run(() -> …)} form for that method, so it
 * still translates exceptions but records no api-usage.
 */
@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.METHOD)
public @interface NoTelemetry {}
