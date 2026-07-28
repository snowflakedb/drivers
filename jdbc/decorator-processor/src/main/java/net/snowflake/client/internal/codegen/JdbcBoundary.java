package net.snowflake.client.internal.codegen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks an implementation class as a JDBC API boundary. The annotation processor generates a
 * decorator that extends {@code AbstractDecorator<AnnotatedClass>}, implements the same interfaces
 * (minus internal ones like {@code DelegatingWrapper}), and delegates every method through {@code
 * map()}/{@code run()} for exception translation.
 *
 * <p>Generated class name: {@code Decorated<ClassName>} in the {@code ...api.decorator} package.
 */
@Retention(RetentionPolicy.SOURCE)
@Target(ElementType.TYPE)
public @interface JdbcBoundary {}
