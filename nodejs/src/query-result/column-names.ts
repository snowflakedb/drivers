import type { RowMode } from './types.js';

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

// TODO: consider resolving column names in nodejs_bridge
export function resolveColumnNames(columns: { getName(): string }[], rowMode: RowMode): string[] {
  const names = columns.map((column) => column.getName());
  return rowMode === 'object_with_renamed_duplicated_columns'
    ? renameDuplicateColumnNames(names)
    : names;
}
