package net.snowflake.client.internal.log;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import java.util.function.Supplier;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.slf4j.helpers.FormattingTuple;
import org.slf4j.helpers.MessageFormatter;

class LogFormatterTest {

  @ParameterizedTest(name = "shouldMatchSlf4jArrayFormat {index}")
  @MethodSource("slf4jParityCases")
  void shouldMatchSlf4jArrayFormat(String pattern, Object[] args) {
    FormattingTuple expected = MessageFormatter.arrayFormat(pattern, args);
    LogFormatter.Formatted actual = LogFormatter.format(pattern, args);
    assertEquals(expected.getMessage(), actual.getMessage());
    assertSame(expected.getThrowable(), actual.getThrowable());
  }

  static Stream<Arguments> slf4jParityCases() {
    RuntimeException cause = new RuntimeException("boom");
    Object[] nested = new Object[] {"a", new int[] {1, 2}};
    return Stream.of(
        Arguments.of(null, new Object[] {"x"}),
        Arguments.of("plain", new Object[0]),
        Arguments.of("hello {}", new Object[] {"world"}),
        Arguments.of("{}-{}", new Object[] {1, 2}),
        Arguments.of("left {} right", new Object[] {null}),
        Arguments.of("no placeholders", new Object[] {"unused"}),
        Arguments.of("one {} two {}", new Object[] {"a"}),
        Arguments.of("literal \\{} and {}", new Object[] {"arg"}),
        Arguments.of("file C:\\\\{}", new Object[] {"x.zip"}),
        Arguments.of("x\\a{}", new Object[] {"Y"}),
        Arguments.of("x\\a{} and {}", new Object[] {"Y", "Z"}),
        Arguments.of("Set {1,2,3} is not {}", new Object[] {"equal"}),
        Arguments.of("only throwable", new Object[] {cause}),
        Arguments.of("failed {}", new Object[] {cause}),
        Arguments.of("failed {} {}", new Object[] {"step", cause}),
        Arguments.of("failed {} {}", new Object[] {"a", "b", cause}),
        Arguments.of("values {}", new Object[] {new int[] {3, 4}}),
        Arguments.of("nested {}", new Object[] {nested}),
        Arguments.of("bools {}", new Object[] {new boolean[] {true, false}}));
  }

  @Test
  void shouldEvaluateSupplierBeforeFormatting() {
    Supplier<String> lazy = () -> "computed";
    LogFormatter.Formatted actual = LogFormatter.format("value {}", lazy);
    FormattingTuple expected = MessageFormatter.arrayFormat("value {}", new Object[] {"computed"});
    assertEquals(expected.getMessage(), actual.getMessage());
    assertNull(actual.getThrowable());
  }

  @Test
  void shouldPrintCyclicArrayWithEllipsis() {
    Object[] cyclic = new Object[] {"a", null};
    cyclic[1] = cyclic;
    LogFormatter.Formatted actual = LogFormatter.format("cycle {}", new Object[] {cyclic});
    FormattingTuple expected = MessageFormatter.arrayFormat("cycle {}", new Object[] {cyclic});
    assertEquals(expected.getMessage(), actual.getMessage());
    assertEquals("cycle [a, [...]]", actual.getMessage());
  }

  @Test
  void shouldPrintSameArrayTwiceWithoutTreatingItAsCycle() {
    Object[] shared = new Object[] {"a"};
    Object[] outer = new Object[] {shared, shared};
    LogFormatter.Formatted actual = LogFormatter.format("twice {}", new Object[] {outer});
    FormattingTuple expected = MessageFormatter.arrayFormat("twice {}", new Object[] {outer});
    assertEquals(expected.getMessage(), actual.getMessage());
    assertEquals("twice [[a], [a]]", actual.getMessage());
  }

  @Test
  void shouldReplaceArgumentWhoseToStringThrowsError() {
    Object failing =
        new Object() {
          @Override
          public String toString() {
            throw new AssertionError("boom");
          }
        };
    LogFormatter.Formatted actual = LogFormatter.format("value {}", failing);
    assertEquals("value [FAILED toString()]", actual.getMessage());
  }

  @Test
  void shouldKeepNullMessageWhenPatternIsNull() {
    LogFormatter.Formatted actual = LogFormatter.format(null);
    assertNull(actual.getMessage());
    assertNull(actual.getThrowable());
  }
}
