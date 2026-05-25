package net.snowflake.jdbc.utils;

import java.lang.reflect.AnnotatedElement;
import org.junit.jupiter.api.extension.ConditionEvaluationResult;
import org.junit.jupiter.api.extension.ExecutionCondition;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * JUnit 5 {@link ExecutionCondition} that enables/disables tests based on whether the universal
 * (new) or legacy (old) JDBC driver is on the classpath.
 *
 * <p>Activated via {@link SkipOldDriver} or {@link SkipNewDriver} annotations.
 */
public class DriverCompatibilityCondition implements ExecutionCondition {

  @Override
  public ConditionEvaluationResult evaluateExecutionCondition(ExtensionContext context) {
    AnnotatedElement element = context.getElement().orElse(null);
    if (element == null) {
      return ConditionEvaluationResult.enabled("No annotation present");
    }

    SkipOldDriver skipOld = element.getAnnotation(SkipOldDriver.class);
    if (skipOld != null && DriverCompatibility.isOldDriver()) {
      return ConditionEvaluationResult.disabled(skipOld.value() + ": Skipped on old driver");
    }

    SkipNewDriver skipNew = element.getAnnotation(SkipNewDriver.class);
    if (skipNew != null && DriverCompatibility.isNewDriver()) {
      return ConditionEvaluationResult.disabled(skipNew.value() + ": Skipped on new driver");
    }

    return ConditionEvaluationResult.enabled("Driver compatibility check passed");
  }
}
