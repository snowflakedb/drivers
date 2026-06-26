package net.snowflake.jdbc.utils;

import java.lang.annotation.Annotation;
import java.util.Optional;
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
    SkipOldDriver skipOld = findAnnotation(context, SkipOldDriver.class);
    if (skipOld != null && DriverCompatibility.isOldDriver()) {
      return ConditionEvaluationResult.disabled(skipOld.value() + ": Skipped on old driver");
    }

    SkipNewDriver skipNew = findAnnotation(context, SkipNewDriver.class);
    if (skipNew != null && DriverCompatibility.isNewDriver()) {
      return ConditionEvaluationResult.disabled(skipNew.value() + ": Skipped on new driver");
    }

    return ConditionEvaluationResult.enabled("Driver compatibility check passed");
  }

  private static <A extends Annotation> A findAnnotation(ExtensionContext ctx, Class<A> type) {
    Optional<A> onElement =
        ctx.getElement().flatMap(element -> Optional.ofNullable(element.getAnnotation(type)));
    if (onElement.isPresent()) {
      return onElement.get();
    }

    Optional<A> onTestClass =
        ctx.getTestClass().flatMap(testClass -> Optional.ofNullable(testClass.getAnnotation(type)));
    if (onTestClass.isPresent()) {
      return onTestClass.get();
    }

    Optional<Class<?>> testClass = ctx.getTestClass();
    if (testClass.isPresent()) {
      Class<?> enclosing = testClass.get().getEnclosingClass();
      while (enclosing != null) {
        A annotation = enclosing.getAnnotation(type);
        if (annotation != null) {
          return annotation;
        }
        enclosing = enclosing.getEnclosingClass();
      }
    }

    return null;
  }
}
