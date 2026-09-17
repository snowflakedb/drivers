package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.condition.EnabledIfEnvironmentVariable;

/**
 * Enables a test only when the CI matrix runs it against the GCP cloud (i.e. {@code
 * CLOUD_PROVIDER=gcp}). Inverse of {@link DisabledOnGCP}: the same env var and case-insensitive
 * matcher, but the test is skipped when the variable is unset or is not GCP.
 *
 * <p>When {@code CLOUD_PROVIDER} is unset — e.g. {@code referenceTest} CI or a local run against a
 * hand-decoded {@code parameters.json} — the test is skipped so an AWS account is not asked to
 * exercise a GCS stage.
 */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@EnabledIfEnvironmentVariable(named = "CLOUD_PROVIDER", matches = "(?i)GCP(?-i)")
public @interface EnabledOnGCP {}
