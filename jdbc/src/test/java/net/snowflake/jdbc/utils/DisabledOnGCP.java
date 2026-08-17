package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable;

/**
 * Disables a test when the CI matrix runs it against the GCP cloud (i.e. {@code
 * CLOUD_PROVIDER=gcp}). Mirrors legacy snowflake-jdbc's {@code RunOnGCP}/{@code RunOnAWS}
 * annotations (same {@code CLOUD_PROVIDER} env var, same case-insensitive matcher), inverted to a
 * disable.
 *
 * <p>When {@code CLOUD_PROVIDER} is unset — e.g. a local run against a hand-decoded {@code
 * parameters.json} — the test is enabled, so this only ever removes coverage in the CI GCP lane.
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@DisabledIfEnvironmentVariable(named = "CLOUD_PROVIDER", matches = "(?i)GCP(?-i)")
public @interface DisabledOnGCP {}
