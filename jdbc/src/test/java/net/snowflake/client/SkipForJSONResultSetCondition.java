package net.snowflake.client;

import org.junit.jupiter.api.extension.ConditionEvaluationResult;
import org.junit.jupiter.api.extension.ExecutionCondition;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * JUnit 5 extension that skips tests annotated with {@link SkipForJSONResultSet} when the
 * QUERY_RESULT_FORMAT environment variable is set to "JSON".
 */
public class SkipForJSONResultSetCondition implements ExecutionCondition {
  private static final String ENV_VAR = "QUERY_RESULT_FORMAT";

  @Override
  public ConditionEvaluationResult evaluateExecutionCondition(ExtensionContext context) {
    String resultFormat = System.getenv(ENV_VAR);
    if ("JSON".equalsIgnoreCase(resultFormat)) {
      String reason =
          context
              .getElement()
              .map(element -> element.getAnnotation(SkipForJSONResultSet.class))
              .map(SkipForJSONResultSet::value)
              .orElse("Test requires Arrow format precision");
      return ConditionEvaluationResult.disabled("Skipped for JSON result format: " + reason);
    }
    return ConditionEvaluationResult.enabled("Not using JSON result format");
  }
}
