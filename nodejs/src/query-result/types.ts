import type {
  CoreColumnInstance,
  CoreKnownSessionParameters,
  CoreSessionStateInstance,
} from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type { RowStatement } from './RowStatement.js';

// TODO: consider converting to an enum -- these string literals are reused
// as bare values across many test cases, which risks silent typos. Public API
// callers pass RowMode as a plain string today, so figure out how to make the
// enum still accept those string literals as input before converting.
export type RowMode = 'array' | 'object' | 'object_with_renamed_duplicated_columns';

// TODO: move this check into the bridge, so it decides from the row mode itself.
export function shouldRenameDuplicateColumns(rowMode: RowMode): boolean {
  return rowMode === 'object_with_renamed_duplicated_columns';
}

export type DataType = 'String' | 'Boolean' | 'Number' | 'Date' | 'JSON' | 'Buffer';

export interface ConverterContext {
  sessionParameters: CoreKnownSessionParameters;
  options: { representNullAsStringNull: boolean };
  onPrecisionLoss: (columnIndex: number) => void;
}

export type ConverterFactory = (
  column: CoreColumnInstance,
  context: ConverterContext,
) => CellConverter;

export type CellConverter = (value: unknown) => unknown;

export type Column = CoreColumnInstance;

export type SessionState = CoreSessionStateInstance;

export interface RowOptions {
  representNullAsStringNull: boolean;
  rowMode: RowMode;
  fetchAsString: DataType[];
  rowStreamHighWaterMark: number;
}

export type StatementCallback = (
  err: SnowflakeError | undefined,
  stmt: RowStatement,
  rows?: any[],
) => void;

export interface StreamOptions {
  start?: number;
  end?: number;
  fetchAsString?: DataType[];
}
export type { QueryStatus, StatementStatus } from '../core/index.js';
