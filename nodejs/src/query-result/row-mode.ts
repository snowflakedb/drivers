// TODO: consider converting to an enum -- these string literals are reused
// as bare values across many test cases, which risks silent typos. Public API
// callers pass RowMode as a plain string today, so figure out how to make the
// enum still accept those string literals as input before converting.
export type RowMode = 'array' | 'object' | 'object_with_renamed_duplicated_columns';

export const DEFAULT_ROW_MODE: RowMode = 'object';

// Only the 2nd+ occurrence of a duplicated name is renamed (suffix _2, _3, ...,
// skipping any name already taken).
export function renameDuplicateColumnNames(names: string[]): string[] {
  const uniqueNames = new Set(names);
  if (names.length === uniqueNames.size) {
    return names;
  }
  const occurrenceCount = new Map<string, number>();
  const result = [...names];
  for (let index = 0; index < names.length; index++) {
    const columnName = names[index];
    // An empty name is never tracked or renamed, even if it repeats.
    if (!columnName) {
      continue;
    }
    if (occurrenceCount.has(columnName)) {
      let times = occurrenceCount.get(columnName)! + 1;
      let candidate = `${columnName}_${times}`;
      while (uniqueNames.has(candidate)) {
        times += 1;
        candidate = `${columnName}_${times}`;
      }
      occurrenceCount.set(columnName, times);
      result[index] = candidate;
      uniqueNames.add(candidate);
    } else {
      occurrenceCount.set(columnName, 1);
    }
  }
  return result;
}

export function resolveColumnNames(columnNames: string[], rowMode: RowMode): string[] {
  return rowMode === 'object_with_renamed_duplicated_columns'
    ? renameDuplicateColumnNames(columnNames)
    : columnNames;
}

// columnNames must already be resolved via resolveColumnNames (renamed once
// per statement, not per row).
export function reshapeRowForMode(
  row: unknown[],
  columnNames: string[],
  rowMode: RowMode,
): unknown {
  if (rowMode === 'array') {
    return row;
  }
  return columnNames.reduce<Record<string, unknown>>((shaped, name, index) => {
    shaped[name] = row[index];
    return shaped;
  }, {});
}
