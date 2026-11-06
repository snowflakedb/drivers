package net.snowflake.client.internal.codegen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a {@link JdbcBoundary} class or method as a hot path that should skip telemetry recording
 * in the generated decorator. The processor then emits a direct delegate call with exception
 * translation only, so the method records no api-usage.
 *
 * <p>A type-level annotation applies to every delegated method on that class. Use that for types
 * that are entirely per-row or per-column ({@code ResultSetMetaData}, {@code ParameterMetaData},
 * {@code Clob}). A method-level annotation covers mixed types ({@code ResultSet} getters, {@code
 * PreparedStatement} bind setters) where {@code execute*} still records.
 */
@Retention(RetentionPolicy.SOURCE)
@Target({ElementType.TYPE, ElementType.METHOD})
public @interface NoTelemetry {}
