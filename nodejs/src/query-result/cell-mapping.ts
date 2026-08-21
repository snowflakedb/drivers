import type { CoreConnectionInstance, CoreColumnInstance } from '../core/index.js';
import SessionParameterName from '../constants/SessionParameterName.js';
import { GlobalConfig } from '../global-config.js';

type CellMapper = (value: unknown) => unknown;

export type RowMapper = (row: unknown[]) => void;

const toNumberMapper: CellMapper = (value) => (value === null ? null : Number(value));
const toBigIntMapper: CellMapper = (value) => (value === null ? null : BigInt(value as string));
const toFloatMapper: CellMapper = (value) => {
  switch (value) {
    case null:
      return null;
    case 'inf':
      return Infinity;
    case '-inf':
      return -Infinity;
    default:
      return Number(value);
  }
};

const variantMapper: CellMapper = (value) => {
  if (value === null || value === undefined) {
    return value;
  }
  if (value === '') {
    return undefined;
  }
  const text = value as string;
  try {
    return GlobalConfig.jsonColumnVariantParser(text);
  } catch {
    return GlobalConfig.xmlColumnVariantParser(text);
  }
};

function isEnabled(connection: CoreConnectionInstance, name: string): boolean {
  return connection.getSessionParameter(name)?.toLowerCase() === 'true';
}

function selectMapper(
  column: CoreColumnInstance,
  connection: CoreConnectionInstance,
): CellMapper | null {
  if (column.isVariant()) {
    return variantMapper;
  }
  if (column.getType() === 'fixed') {
    return isEnabled(connection, SessionParameterName.JS_TREAT_INTEGER_AS_BIGINT) &&
      column.getScale() === 0
      ? toBigIntMapper
      : toNumberMapper;
  }
  if (column.getType() === 'real') {
    return toFloatMapper;
  }
  return null;
}

/** A column that needs conversion, paired with the mapper selected for it. */
interface MappedColumn {
  index: number;
  map: CellMapper;
}

export function createRowMapper(
  columns: CoreColumnInstance[],
  connection: CoreConnectionInstance,
): RowMapper {
  const mapped: MappedColumn[] = [];
  for (const column of columns) {
    const map = selectMapper(column, connection);
    if (map !== null) {
      mapped.push({ index: column.getIndex(), map });
    }
  }

  // Rewrites in place, visiting only the columns that need it: the bridge builds
  // a fresh row array per call so nothing else holds a reference
  return (row) => {
    for (const { index, map } of mapped) {
      row[index] = map(row[index]);
    }
  };
}
