package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.Tag;
import org.junit.jupiter.api.condition.EnabledIfEnvironmentVariable;

/**
 * Marks a test that requires the headless browser Docker container (Chromium + Playwright).
 *
 * <p>Combines two concerns:
 *
 * <ul>
 *   <li>{@code @Tag("requires_browser")} — allows Gradle to include/exclude via {@code
 *       GRADLE_INCLUDE_TAGS=requires_browser}
 *   <li>{@code @EnabledIfEnvironmentVariable} — skips the test at runtime when not inside the
 *       browser container (safety net)
 * </ul>
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@Tag("requires_browser")
@EnabledIfEnvironmentVariable(named = "SF_TEST_HEADLESS_BROWSER", matches = "true")
public @interface RequiresBrowser {}
