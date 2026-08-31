import type { CoreConnectionInstance, CoreColumnInstance } from '../core/index.js';
import type { DataType } from './types.js';
import SessionParameterName from '../constants/SessionParameterName.js';
import { GlobalConfig } from '../global-config.js';

type CellMapper = (value: unknown) => unknown;

export type RowMapper = (row: unknown[]) => void;

const NULL_AS_STRING = 'NULL';

const toNumberMapper: CellMapper = (value) => (value === null ? null : Number(value));
const toBigIntMapper: CellMapper = (value) => (value === null ? null : BigInt(value as string));

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

// TODO: measure building these strings in the bridge instead of here
const textAsStringMapper: CellMapper = (value) => (value === null ? NULL_AS_STRING : value);

const booleanAsStringMapper: CellMapper = (value) => {
  if (value === null) {
    return NULL_AS_STRING;
  }
  return value === true ? 'TRUE' : 'FALSE';
};

const realAsStringMapper: CellMapper = (value) => {
  switch (value) {
    case null:
      return NULL_AS_STRING;
    case Infinity:
      return 'inf';
    case -Infinity:
      return '-inf';
    default:
      return String(value);
  }
};

// TODO: honor BINARY_OUTPUT_FORMAT=BASE64 once session parameters are read from
// the server response; hex is the default and all that is reachable today.
const binaryAsStringMapper: CellMapper = (value) =>
  value === null ? NULL_AS_STRING : (value as Buffer).toString('hex').toUpperCase();

// TODO: honor a non-default DATE_OUTPUT_FORMAT once session parameters are read
// from the server response; YYYY-MM-DD is the default and all that is reachable
// today.
const dateAsStringMapper: CellMapper = (value) =>
  value === null ? NULL_AS_STRING : (value as Date).toISOString().slice(0, 'YYYY-MM-DD'.length);

const TOKEN_BY_COLUMN_TYPE: Record<string, DataType> = {
  text: 'String',
  fixed: 'Number',
  real: 'Number',
  boolean: 'Boolean',
  binary: 'Buffer',
  date: 'Date',
  variant: 'JSON',
  decfloat: 'String',
};

function toTokenSet(fetchAsString?: DataType[]): ReadonlySet<string> {
  return new Set(fetchAsString?.map((token) => token.toUpperCase()));
}

/** A column type with no token has nothing to render, so it is never selected. */
function isRequestedAsString(column: CoreColumnInstance, tokens: ReadonlySet<string>): boolean {
  const token = TOKEN_BY_COLUMN_TYPE[column.getType()];
  return token !== undefined && tokens.has(token.toUpperCase());
}

function selectAsStringMapper(column: CoreColumnInstance): CellMapper | null {
  if (column.isBoolean()) {
    return booleanAsStringMapper;
  }
  if (column.getType() === 'real') {
    return realAsStringMapper;
  }
  if (column.isBinary()) {
    return binaryAsStringMapper;
  }
  if (column.isDate()) {
    return dateAsStringMapper;
  }
  return textAsStringMapper;
}

function isEnabled(connection: CoreConnectionInstance, name: string): boolean {
  return connection.getSessionParameter(name)?.toLowerCase() === 'true';
}

function selectNormalMapper(
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
  return null;
}

function selectMapper(
  column: CoreColumnInstance,
  connection: CoreConnectionInstance,
  asStringTokens: ReadonlySet<string>,
): CellMapper | null {
  return isRequestedAsString(column, asStringTokens)
    ? selectAsStringMapper(column)
    : selectNormalMapper(column, connection);
}

/** A column that needs conversion, paired with the mapper selected for it. */
interface MappedColumn {
  index: number;
  map: CellMapper;
}

export function createRowMapper(
  columns: CoreColumnInstance[],
  connection: CoreConnectionInstance,
  fetchAsString?: DataType[],
): RowMapper {
  const asStringTokens = toTokenSet(fetchAsString);
  const mapped: MappedColumn[] = [];
  for (const column of columns) {
    const map = selectMapper(column, connection, asStringTokens);
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
