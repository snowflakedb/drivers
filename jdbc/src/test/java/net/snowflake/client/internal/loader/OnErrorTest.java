package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

class OnErrorTest {

  @ParameterizedTest
  @ValueSource(
      strings = {
        "ABORT_STATEMENT",
        "ABORT_STATeMENT",
        "CONTINUE",
        "cOnTinue",
        "SKIP_FILE",
        "Skip_File_10",
        // Out of range values are rejected by the server, not here.
        "Skip_File_123%"
      })
  void shouldAcceptValidOnErrorValue(String value) {
    assertTrue(OnError.validate(value));
  }

  @ParameterizedTest
  @NullAndEmptySource
  @ValueSource(
      strings = {
        "Bogus",
        "abortstatement",
        "continues",
        "skipfile",
        "skip_file_abcdef",
        "skip_file_abcdef%",
        "skip_file_25.0%",
        "skip_file_-10%",
        "continue; delete from t",
        "ABORT_STATEMENT; drop table t"
      })
  void shouldRejectInvalidOnErrorValue(String value) {
    assertFalse(OnError.validate(value));
  }
}
