package net.snowflake.client.internal.api.implementation.exception;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import org.junit.jupiter.api.Test;

public class SnowflakeSQLExceptionConstructorsTest {

  private static final ErrorCode CODE = ErrorCode.INVALID_PARAMETER_TYPE;

  @Test
  public void shouldRenderTemplateWhenVendorCodeMatchesAnErrorCode() {
    SnowflakeSQLException e =
        new SnowflakeSQLException(CODE.getSqlState(), CODE.getMessageCode(), "a", "b");

    assertEquals(CODE.getSqlState(), e.getSQLState());
    assertEquals(CODE.getMessageCode(), e.getErrorCode());
    assertEquals("Invalid parameter value type: a, expected type: b.", e.getMessage());
    assertNull(e.getQueryId());
  }

  @Test
  public void shouldFallBackToVendorCodeAndParamsWhenNoErrorCodeMatches() {
    SnowflakeSQLException e = new SnowflakeSQLException("42000", 999999, "x", 7);

    assertEquals("999999: [x, 7]", e.getMessage());
    assertEquals("42000", e.getSQLState());
    assertEquals(999999, e.getErrorCode());
  }

  @Test
  public void shouldUseBareVendorCodeAsMessageWhenThereAreNoParams() {
    SnowflakeSQLException e = new SnowflakeSQLException("qid-1", "42000", 999999);

    assertEquals("999999", e.getMessage());
    assertEquals("qid-1", e.getQueryId());
    assertEquals("42000", e.getSQLState());
    assertEquals(999999, e.getErrorCode());
  }

  @Test
  public void shouldCarryQueryIdReasonAndSqlStateWithZeroVendorCode() {
    SnowflakeSQLException e = new SnowflakeSQLException("qid-2", "boom", "XX000");

    assertEquals("boom", e.getMessage());
    assertEquals("XX000", e.getSQLState());
    assertEquals(0, e.getErrorCode());
    assertEquals("qid-2", e.getQueryId());
  }

  @Test
  public void shouldCarryQueryIdReasonSqlStateAndVendorCode() {
    SnowflakeSQLException e = new SnowflakeSQLException("qid-3", "boom", "XX000", 1234);

    assertEquals("boom", e.getMessage());
    assertEquals("XX000", e.getSQLState());
    assertEquals(1234, e.getErrorCode());
    assertEquals("qid-3", e.getQueryId());
  }

  @Test
  public void shouldCarryCauseWithQueryIdAndVendorCodeParams() {
    Throwable cause = new RuntimeException("root");

    SnowflakeSQLException e =
        new SnowflakeSQLException("qid-4", cause, CODE.getSqlState(), CODE.getMessageCode(), "p");

    assertSame(cause, e.getCause());
    assertEquals("qid-4", e.getQueryId());
    assertEquals(CODE.getMessageCode(), e.getErrorCode());
  }

  @Test
  public void shouldCarryQueryIdWithErrorCodeAndParams() {
    SnowflakeSQLException e = new SnowflakeSQLException("qid-5", CODE, "a", "b");

    assertEquals("qid-5", e.getQueryId());
    assertEquals(CODE.getSqlState(), e.getSQLState());
    assertEquals(CODE.getMessageCode(), e.getErrorCode());
    assertEquals("Invalid parameter value type: a, expected type: b.", e.getMessage());
  }

  @Test
  public void shouldAcceptCauseBeforeMessage() {
    Throwable cause = new RuntimeException("root");

    SnowflakeSQLException e = new SnowflakeSQLException(cause, "wrapped");

    assertEquals("wrapped", e.getMessage());
    assertSame(cause, e.getCause());
    assertNull(e.getSQLState());
  }

  @Test
  public void shouldCarryCauseWithSqlStateAndVendorCode() {
    Throwable cause = new RuntimeException("root");

    SnowflakeSQLException bare = new SnowflakeSQLException(cause, "42000", 999999);
    SnowflakeSQLException withParams = new SnowflakeSQLException(cause, "42000", 999999, "x");

    assertEquals("999999", bare.getMessage());
    assertEquals("999999: [x]", withParams.getMessage());
    assertSame(cause, bare.getCause());
    assertSame(cause, withParams.getCause());
    assertEquals(999999, withParams.getErrorCode());
    assertEquals("42000", withParams.getSQLState());
  }

  @Test
  public void shouldCarryCauseWithErrorCodeAndParams() {
    Throwable cause = new RuntimeException("root");

    SnowflakeSQLException e = new SnowflakeSQLException(cause, CODE, "a", "b");

    assertSame(cause, e.getCause());
    assertEquals(CODE.getSqlState(), e.getSQLState());
    assertEquals(CODE.getMessageCode(), e.getErrorCode());
    assertEquals("Invalid parameter value type: a, expected type: b.", e.getMessage());
  }

  @Test
  public void shouldRenderErrorCodeTemplateWhenOnlyParamsFollowErrorCode() {
    SnowflakeSQLException e = new SnowflakeSQLException(CODE, 1, 2);

    assertEquals("Invalid parameter value type: 1, expected type: 2.", e.getMessage());
    assertEquals(CODE.getSqlState(), e.getSQLState());
    assertEquals(CODE.getMessageCode(), e.getErrorCode());
    assertNull(e.getQueryId());
  }
}
