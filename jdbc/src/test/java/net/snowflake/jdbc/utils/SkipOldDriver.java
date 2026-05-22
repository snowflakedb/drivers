package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.extension.ExtendWith;

/**
 * Skips the annotated test when running against the old (legacy) JDBC driver. Use this for tests
 * that exercise new-driver-only behavior documented in jdbc/BehaviorDifferences.yaml.
 *
 * <p>Usage:
 *
 * <pre>{@code
 * @Test
 * @SkipOldDriver("BD#1")
 * void myNewDriverOnlyTest() { ... }
 * }</pre>
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@ExtendWith(DriverCompatibilityCondition.class)
public @interface SkipOldDriver {
  /** The behavioral difference ID (e.g. "BD#1") that documents why this test is skipped. */
  String value();
}
