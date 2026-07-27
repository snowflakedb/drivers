package net.snowflake.jdbc.utils;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.Tag;
import org.junit.jupiter.api.condition.EnabledIfEnvironmentVariable;

/** Marks a test gated by {@code requires_no_mfa} / {@code SF_TEST_NO_MFA=true}. */
@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@Tag("requires_no_mfa")
@EnabledIfEnvironmentVariable(named = "SF_TEST_NO_MFA", matches = "true")
public @interface RequiresNoMfa {}
