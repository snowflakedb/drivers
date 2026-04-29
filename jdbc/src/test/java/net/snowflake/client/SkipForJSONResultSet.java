package net.snowflake.client;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.extension.ExtendWith;

/**
 * Marks a test to be skipped when QUERY_RESULT_FORMAT=JSON is set.
 *
 * <p>Use this annotation for tests that rely on Arrow-specific behavior or precision that JSON
 * format cannot provide.
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@ExtendWith(SkipForJSONResultSetCondition.class)
public @interface SkipForJSONResultSet {
  String value() default "Test requires Arrow format";
}
