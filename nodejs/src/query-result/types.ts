import type { CoreColumnInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type { RowStatement, FileAndStageBindStatement } from './RowStatement.js';

// TODO: consider converting to an enum -- these string literals are reused
// as bare values across many test cases, which risks silent typos. Public API
// callers pass RowMode as a plain string today, so figure out how to make the
// enum still accept those string literals as input before converting.
export type RowMode = 'array' | 'object' | 'object_with_renamed_duplicated_columns';

export type DataType = 'String' | 'Boolean' | 'Number' | 'Date' | 'JSON' | 'Buffer';

export interface ConversionContext {
  scale: number | null;
  treatIntegerAsBigInt: boolean;
}

export type CellConverter = (value: unknown, context: ConversionContext) => unknown;

export type Column = CoreColumnInstance;

export interface SessionParameters {
  treatIntegerAsBigInt: boolean;
}

export interface RowOptions {
  rowMode?: RowMode;
  fetchAsString?: DataType[];
}

export type StatementCallback = (
  err: SnowflakeError | undefined,
  stmt: RowStatement | FileAndStageBindStatement,
  rows: Array<unknown> | undefined,
) => void;

export interface StreamOptions {
  start?: number;
  end?: number;
  fetchAsString?: DataType[];
}

export interface FetchRowsOptions {
  each: (row: unknown) => boolean | void;
  end: (err: SnowflakeError | undefined, stmt: RowStatement | FileAndStageBindStatement) => void;
}
