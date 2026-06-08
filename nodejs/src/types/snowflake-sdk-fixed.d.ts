// Patches known gaps in the upstream `snowflake-sdk` `.d.ts`. Module
// augmentation: once this file is in the TS program (via tsconfig `include`),
// every `from 'snowflake-sdk'` import in the project sees the patched types.
import type { QueryStatus } from 'snowflake-sdk';
import ColumnType from '../constants/ColumnType';

declare module 'snowflake-sdk' {
  // TODO:
  // These constants are exported by the old driver but are undocumented.
  // We declare them here and implement them in the new driver for compatibility.
  // Might remove them later or move to a separate namespace.
  export const STRING = ColumnType.STRING;
  export const BOOLEAN = ColumnType.BOOLEAN;
  export const NUMBER = ColumnType.NUMBER;
  export const DATE = ColumnType.DATE;
  export const OBJECT = ColumnType.OBJECT;
  export const ARRAY = ColumnType.ARRAY;
  export const MAP = ColumnType.MAP;
  export const JSON = ColumnType.JSON;

  interface Connection {
    // Upstream: `getQueryStatus(queryId: string): Promise<string>`.
    // The server only ever returns one of the `QueryStatus` literals, and
    // `isStillRunning(status: QueryStatus)` refuses a plain `string`, so every
    // caller had to cast. Narrow the return type here once.
    getQueryStatus(queryId: string): Promise<QueryStatus>;

    // Upstream: `getQueryStatusThrowIfError(queryId: string): Promise<string>`.
    // Same story as `getQueryStatus` — same enum, same downstream APIs reject
    // `string`. Narrowed in lockstep.
    getQueryStatusThrowIfError(queryId: string): Promise<QueryStatus>;

    // Upstream: `isAnError(): boolean` (zero-arg).
    // The runtime implementation actually takes a status string and returns
    // whether that status represents an error. The 0-arg signature in the
    // `.d.ts` is just wrong; fix the arity and tighten the arg to `QueryStatus`
    // (which is what every realistic caller already has on hand).
    isAnError(status: QueryStatus): boolean;
  }
}
