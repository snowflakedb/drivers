export type Bind = string | number | boolean | null;
export type InsertBinds = readonly Bind[][];
export type Binds = readonly Bind[] | InsertBinds;

type SnowflakeBindType = 'BOOLEAN' | 'VARIANT' | 'FIXED' | 'REAL' | 'TEXT';
type BindText = string | null;

interface ServerBind {
  type: SnowflakeBindType;
  value: BindText | BindText[];
}

function isMultiRowBinding(binds: Binds): binds is InsertBinds {
  return binds.length > 0 && Array.isArray(binds[0]);
}

// This maps every value to a coarse set of logical bind types (BOOLEAN, VARIANT,
// FIXED, REAL, TEXT) and leaves the server to recast the stringified value into the
// column's final type. It matches the old driver, but the Python and JDBC drivers bind
// dedicated types (DATE, TIME, TIMESTAMP_*, BINARY) with epoch-based encodings, which
// avoids that server-side recast. A JS Date binds as VARIANT here, not a temporal type.
// Aligning with the other drivers is tracked as a TODO in BCR_LOG.md.
function snowflakeTypeOf(value: unknown): SnowflakeBindType {
  if (typeof value === 'boolean') return 'BOOLEAN';
  if (value !== null && typeof value === 'object') return 'VARIANT';
  if (typeof value === 'number') return Number.isInteger(value) ? 'FIXED' : 'REAL';
  return 'TEXT';
}

function bindValueToText(value: unknown): BindText {
  // null stays null so `SET name = :1` binds a SQL NULL, not the literal 'null'.
  if (value === null) return null;
  if (typeof value === 'string') return value;
  if (value instanceof Date) return value.toJSON();
  return JSON.stringify(value);
}

export function buildBindsMap(binds: Binds): Record<string, ServerBind> {
  const rows: readonly (readonly Bind[])[] = isMultiRowBinding(binds) ? binds : [binds];
  const columnCount = rows[0]?.length ?? 0;
  const multiRow = isMultiRowBinding(binds);

  const bindMap: Record<string, ServerBind> = {};
  for (let column = 0; column < columnCount; column++) {
    const valuesForColumn = rows.map((row) => row[column]);
    bindMap[String(column + 1)] = {
      type: snowflakeTypeOf(valuesForColumn[0]),
      value: multiRow ? valuesForColumn.map(bindValueToText) : bindValueToText(valuesForColumn[0]),
    };
  }
  return bindMap;
}

export function countBoundValues(binds?: Binds): number {
  if (!Array.isArray(binds)) return 0;
  let total = 0;
  // Scalar (single-row) binds contribute 0, which keeps them off the stage path;
  // only bulk array binds can exceed the threshold.
  for (const row of binds) {
    if (Array.isArray(row)) total += row.length;
  }
  return total;
}
