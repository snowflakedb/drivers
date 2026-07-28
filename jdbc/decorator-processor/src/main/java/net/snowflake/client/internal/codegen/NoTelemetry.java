package net.snowflake.client.internal.codegen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a method on a {@link JdbcBoundary}-annotated class as a hot-path accessor that should skip
 * telemetry recording in the generated decorator.
 */
@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.METHOD)
public @interface NoTelemetry {}
