package net.snowflake.client.internal.api.implementation.statement;

import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.SortedSet;
import java.util.TreeSet;

final class SqlPlaceholderMetadata {
  enum PlaceholderStyle {
    NONE,
    POSITIONAL,
    NUMERIC,
    MIXED
  }

  private final PlaceholderStyle placeholderStyle;
  private final int positionalPlaceholderCount;
  private final List<Integer> referencedParameterIndexes;
  private final SortedSet<Integer> referencedNumericParameterIndexes;

  private SqlPlaceholderMetadata(
      PlaceholderStyle placeholderStyle,
      int positionalPlaceholderCount,
      List<Integer> referencedParameterIndexes,
      SortedSet<Integer> referencedNumericParameterIndexes) {
    this.placeholderStyle = placeholderStyle;
    this.positionalPlaceholderCount = positionalPlaceholderCount;
    this.referencedParameterIndexes = Collections.unmodifiableList(referencedParameterIndexes);
    this.referencedNumericParameterIndexes =
        Collections.unmodifiableSortedSet(referencedNumericParameterIndexes);
  }

  static SqlPlaceholderMetadata analyze(String sql) {
    // TODO: Replace this with a proper SQL-aware analyzer that skips strings, comments, and other
    // syntax edge cases. For now we only support the simple placeholder patterns covered by tests.
    int positionalPlaceholderCount = 0;
    SortedSet<Integer> numericParameterIndexes = new TreeSet<>();

    for (int i = 0; i < sql.length(); i++) {
      char currentChar = sql.charAt(i);

      if (currentChar == '?') {
        positionalPlaceholderCount++;
        continue;
      }

      if (currentChar == ':'
          && hasNextChar(sql, i)
          && Character.isDigit(sql.charAt(i + 1))
          && (i == 0 || sql.charAt(i - 1) != ':')) {
        int placeholderEnd = i + 1;
        while (placeholderEnd < sql.length() && Character.isDigit(sql.charAt(placeholderEnd))) {
          placeholderEnd++;
        }
        numericParameterIndexes.add(Integer.parseInt(sql.substring(i + 1, placeholderEnd)));
        i = placeholderEnd - 1;
      }
    }

    PlaceholderStyle placeholderStyle =
        determinePlaceholderStyle(positionalPlaceholderCount, numericParameterIndexes);
    List<Integer> referencedParameterIndexes =
        buildReferencedParameterIndexes(
            positionalPlaceholderCount, numericParameterIndexes, placeholderStyle);

    return new SqlPlaceholderMetadata(
        placeholderStyle,
        positionalPlaceholderCount,
        referencedParameterIndexes,
        numericParameterIndexes);
  }

  PlaceholderStyle placeholderStyle() {
    return placeholderStyle;
  }

  boolean hasBindings() {
    return placeholderStyle != PlaceholderStyle.NONE;
  }

  boolean hasMixedPlaceholderStyles() {
    return placeholderStyle == PlaceholderStyle.MIXED;
  }

  int placeholderCount() {
    return referencedParameterIndexes.size();
  }

  int positionalPlaceholderCount() {
    return positionalPlaceholderCount;
  }

  Collection<Integer> referencedParameterIndexes() {
    return referencedParameterIndexes;
  }

  boolean referencesParameterIndex(int parameterIndex) {
    return referencedNumericParameterIndexes.contains(parameterIndex)
        || (placeholderStyle == PlaceholderStyle.POSITIONAL
            && parameterIndex <= positionalPlaceholderCount);
  }

  private static PlaceholderStyle determinePlaceholderStyle(
      int positionalPlaceholderCount, SortedSet<Integer> numericParameterIndexes) {
    if (positionalPlaceholderCount == 0 && numericParameterIndexes.isEmpty()) {
      return PlaceholderStyle.NONE;
    }
    if (positionalPlaceholderCount > 0 && !numericParameterIndexes.isEmpty()) {
      return PlaceholderStyle.MIXED;
    }
    if (positionalPlaceholderCount > 0) {
      return PlaceholderStyle.POSITIONAL;
    }
    return PlaceholderStyle.NUMERIC;
  }

  private static List<Integer> buildReferencedParameterIndexes(
      int positionalPlaceholderCount,
      SortedSet<Integer> numericParameterIndexes,
      PlaceholderStyle placeholderStyle) {
    if (placeholderStyle == PlaceholderStyle.POSITIONAL) {
      List<Integer> referencedParameterIndexes = new ArrayList<>(positionalPlaceholderCount);
      for (int parameterIndex = 1; parameterIndex <= positionalPlaceholderCount; parameterIndex++) {
        referencedParameterIndexes.add(parameterIndex);
      }
      return referencedParameterIndexes;
    }
    return new ArrayList<>(numericParameterIndexes);
  }

  private static boolean hasNextChar(String sql, int index) {
    return index + 1 < sql.length();
  }
}
