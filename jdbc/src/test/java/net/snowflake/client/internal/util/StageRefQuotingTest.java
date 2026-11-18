package net.snowflake.client.internal.util;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import org.junit.jupiter.api.Test;

class StageRefQuotingTest {

  @Test
  void shouldLeaveAsciiStageRefUnquoted() {
    assertEquals("@my_stage", StageRefQuoting.quoteIfNeeded("@my_stage"));
    assertEquals("@my_stage/dir/file.csv", StageRefQuoting.quoteIfNeeded("@my_stage/dir/file.csv"));
    assertEquals("@~/dir/file.csv", StageRefQuoting.quoteIfNeeded("@~/dir/file.csv"));
    assertEquals(
        "@\"DB\".\"SCHEMA\".STAGE", StageRefQuoting.quoteIfNeeded("@\"DB\".\"SCHEMA\".STAGE"));
  }

  @Test
  void shouldQuoteNonAsciiStageRef() {
    assertEquals("'@\"DB\".\"日本語\".STAGE'", StageRefQuoting.quoteIfNeeded("@\"DB\".\"日本語\".STAGE"));
    assertEquals("'@\"café\".STAGE'", StageRefQuoting.quoteIfNeeded("@\"café\".STAGE"));
  }

  @Test
  void shouldQuoteStageRefContainingSpace() {
    assertEquals(
        "'@\"my stage\"/path/file.csv'",
        StageRefQuoting.quoteIfNeeded("@\"my stage\"/path/file.csv"));
  }

  @Test
  void shouldDoubleEmbeddedSingleQuote() {
    assertEquals("'@stage_O''Brien'", StageRefQuoting.quoteIfNeeded("@stage_O'Brien"));
    assertEquals("'@\"sch''ema\".STAGE'", StageRefQuoting.quoteIfNeeded("@\"sch'ema\".STAGE"));
  }

  @Test
  void shouldLeaveAlreadyQuotedStageRefUnchanged() {
    String singleQuoted = "'@\"DB\".\"日本語\".STAGE'";
    assertEquals(singleQuoted, StageRefQuoting.quoteIfNeeded(singleQuoted));
    String dollarQuoted = "$$@\"DB\".\"日本語\".STAGE$$";
    assertEquals(dollarQuoted, StageRefQuoting.quoteIfNeeded(dollarQuoted));
    assertEquals("'@stage_O''Brien'", StageRefQuoting.quoteIfNeeded("'@stage_O''Brien'"));
  }

  @Test
  void shouldLeaveNullStageRefNull() {
    assertNull(StageRefQuoting.quoteIfNeeded(null));
    assertNull(StageRefQuoting.unwrapOuterQuotes(null));
  }

  @Test
  void shouldUnwrapSingleQuotedStageRef() {
    assertEquals(
        "@\"DB\".\"日本語\".STAGE", StageRefQuoting.unwrapOuterQuotes("'@\"DB\".\"日本語\".STAGE'"));
    assertEquals("@stage_O'Brien", StageRefQuoting.unwrapOuterQuotes("'@stage_O''Brien'"));
  }

  @Test
  void shouldUnwrapDollarQuotedStageRef() {
    assertEquals(
        "@\"DB\".\"日本語\".STAGE", StageRefQuoting.unwrapOuterQuotes("$$@\"DB\".\"日本語\".STAGE$$"));
  }

  @Test
  void shouldAddAtPrefixToStageRefWithoutOne() {
    assertEquals("@MYSTAGE", StageRefQuoting.withAtPrefix("MYSTAGE"));
    assertEquals("@~", StageRefQuoting.withAtPrefix("~"));
    assertEquals("@DB.PUBLIC.MYSTAGE", StageRefQuoting.withAtPrefix("DB.PUBLIC.MYSTAGE"));
    assertEquals(
        "@\"DB\".\"SCHEMA\".STAGE", StageRefQuoting.withAtPrefix("\"DB\".\"SCHEMA\".STAGE"));
  }

  @Test
  void shouldLeaveStageRefWithAtPrefixUnchanged() {
    assertEquals("@my_stage", StageRefQuoting.withAtPrefix("@my_stage"));
    assertEquals("@~", StageRefQuoting.withAtPrefix("@~"));
    String singleQuoted = "'@\"DB\".\"日本語\".STAGE'";
    assertEquals(singleQuoted, StageRefQuoting.withAtPrefix(singleQuoted));
    String dollarQuoted = "$$@\"DB\".\"日本語\".STAGE$$";
    assertEquals(dollarQuoted, StageRefQuoting.withAtPrefix(dollarQuoted));
  }

  @Test
  void shouldAddAtPrefixInsideQuoteWrapper() {
    assertEquals("'@MYSTAGE'", StageRefQuoting.withAtPrefix("'MYSTAGE'"));
    assertEquals("$$@MYSTAGE$$", StageRefQuoting.withAtPrefix("$$MYSTAGE$$"));
    assertEquals("'@stage_O''Brien'", StageRefQuoting.withAtPrefix("'stage_O''Brien'"));
  }

  @Test
  void shouldLeaveNullStageRefNullWithAtPrefix() {
    assertNull(StageRefQuoting.withAtPrefix(null));
  }

  @Test
  void shouldLeaveUnquotedAndMalformedStageRefUnchanged() {
    assertEquals("@my_stage", StageRefQuoting.unwrapOuterQuotes("@my_stage"));
    assertEquals("'a'b'c'", StageRefQuoting.unwrapOuterQuotes("'a'b'c'"));
  }
}
