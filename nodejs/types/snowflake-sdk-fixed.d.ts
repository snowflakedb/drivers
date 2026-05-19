// Patches known gaps in the upstream `snowflake-sdk` `.d.ts`. Module
// augmentation: once this file is in the TS program (via tsconfig `include`),
// every `from 'snowflake-sdk'` import in the project sees the patched types.
import type { QueryStatus } from 'snowflake-sdk';

declare module 'snowflake-sdk' {
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
