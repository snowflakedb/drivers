package net.snowflake.client.internal.codegen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks an implementation class as a JDBC API boundary. The annotation processor generates a
 * decorator that extends {@code AbstractDecorator<AnnotatedClass>}, implements the same interfaces
 * (minus internal ones like {@code DelegatingWrapper}), and delegates every method.
 *
 * <p>By default each method goes through {@code call()}/{@code run()} for exception translation and
 * api-usage telemetry. {@link NoTelemetry} on the type skips telemetry for every delegated method.
 * {@link NoTelemetry} on a method skips telemetry for that method only. Exception translation still
 * applies in both cases.
 *
 * <p>Generated class name: {@code Decorated<ClassName>} in the impl's own package.
 */
@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.TYPE)
public @interface JdbcBoundary {}
