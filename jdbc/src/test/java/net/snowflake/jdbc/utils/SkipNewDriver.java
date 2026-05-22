package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.extension.ExtendWith;

/**
 * Skips the annotated test when running against the new (universal) JDBC driver. Use this for tests
 * that verify old-driver-specific behavior documented in jdbc/BehaviorDifferences.yaml.
 *
 * <p>Usage:
 *
 * <pre>{@code
 * @Test
 * @SkipNewDriver("BD#1")
 * void myOldDriverOnlyTest() { ... }
 * }</pre>
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@ExtendWith(DriverCompatibilityCondition.class)
public @interface SkipNewDriver {
  /** The behavioral difference ID (e.g. "BD#1") that documents why this test is skipped. */
  String value();
}
