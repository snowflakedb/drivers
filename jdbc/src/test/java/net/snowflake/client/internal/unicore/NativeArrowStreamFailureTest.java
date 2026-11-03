package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.sql.SQLException;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;

class NativeArrowStreamFailureTest {

  @Test
  void shouldRejectNullStreamPointerAsInternalError() {
    SFSQLException thrown = assertThrows(SFSQLException.class, () -> NativeArrowStream.open(0L));

    SQLException sql = thrown.toSQLException();
    assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), sql.getErrorCode());
    assertEquals("Arrow stream pointer is null or misaligned", sql.getMessage());
  }

  @Test
  void shouldTranslateUnsupportedColumnTagToFeatureUnsupported() {
    String detail = "Arrow column \"C\" is unsupported: dictionary-encoded columns";
    IllegalStateException nativeFailure = new IllegalStateException("UNSUPPORTED_COLUMN:" + detail);
    SFSQLException translated = NativeArrowStream.arrowFailure(nativeFailure);

    SQLException sql = translated.toSQLException();
    assertEquals(ErrorCode.FEATURE_UNSUPPORTED.getMessageCode(), sql.getErrorCode());
    assertEquals("0A000", sql.getSQLState());
    assertEquals(detail, sql.getMessage());
    assertSame(nativeFailure, translated.getCause());
  }

  @Test
  void shouldTranslateUntaggedUnsupportedWordingToInternalError() {
    IllegalStateException nativeFailure =
        new IllegalStateException("Arrow column \"C\" is unsupported: dictionary-encoded columns");
    SFSQLException translated = NativeArrowStream.arrowFailure(nativeFailure);

    SQLException sql = translated.toSQLException();
    assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), sql.getErrorCode());
    assertEquals(nativeFailure.getMessage(), sql.getMessage());
    assertSame(nativeFailure, translated.getCause());
  }

  @Test
  void shouldTranslateGenericNativeFailureToInternalError() {
    IllegalStateException nativeFailure = new IllegalStateException("Arrow stream error: boom");
    SFSQLException translated = NativeArrowStream.arrowFailure(nativeFailure);

    SQLException sql = translated.toSQLException();
    assertEquals(ErrorCode.INTERNAL_ERROR.getMessageCode(), sql.getErrorCode());
    assertEquals("Arrow stream error: boom", sql.getMessage());
    assertSame(nativeFailure, translated.getCause());
  }

  @Test
  void shouldUseFallbackMessageWhenNativeFailureHasNone() {
    SFSQLException translated =
        NativeArrowStream.arrowFailure(new IllegalStateException((String) null));
    assertEquals("Arrow stream failed", translated.getMessage());
  }
}
