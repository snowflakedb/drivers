package net.snowflake.client;

import java.util.Optional;
import org.junit.jupiter.api.extension.ConditionEvaluationResult;
import org.junit.jupiter.api.extension.ExecutionCondition;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * JUnit 5 extension that skips tests annotated with {@link SkipForJSONResultSet} when the
 * QUERY_RESULT_FORMAT environment variable is set to "JSON" (case-insensitive).
 */
public class SkipForJSONResultSetCondition implements ExecutionCondition {
  private static final String ENV_VAR = "QUERY_RESULT_FORMAT";

  @Override
  public ConditionEvaluationResult evaluateExecutionCondition(ExtensionContext context) {
    String resultFormat = System.getenv(ENV_VAR);
    if ("JSON".equalsIgnoreCase(resultFormat)) {
      // Check method-level annotation first, then class-level
      SkipForJSONResultSet annotation =
          context
              .getElement()
              .flatMap(
                  element -> Optional.ofNullable(element.getAnnotation(SkipForJSONResultSet.class)))
              .orElseGet(
                  () ->
                      context
                          .getTestClass()
                          .flatMap(
                              testClass ->
                                  Optional.ofNullable(
                                      testClass.getAnnotation(SkipForJSONResultSet.class)))
                          .orElse(null));

      String reason = annotation != null ? annotation.value() : "Test works on Arrow only";
      return ConditionEvaluationResult.disabled("Skipped for JSON result format. " + reason);
    }
    return ConditionEvaluationResult.enabled("Not using JSON result format");
  }
}
