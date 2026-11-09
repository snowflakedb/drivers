import type { DataType, RowMode, StatementCallback } from '../query-result/types.js';

export interface FetchResultOptions {
  queryId: string;
  complete?: StatementCallback;
  streamResult?: boolean;
  rowMode?: RowMode;
  fetchAsString?: DataType[];
}
