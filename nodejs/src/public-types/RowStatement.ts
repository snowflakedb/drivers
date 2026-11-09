import type { Readable } from 'node:stream';
import type {
  SessionState,
  StatementCallback,
  StatementStatus,
  StreamOptions,
} from '../query-result/types.js';
import type { Column } from './Column.js';

export interface RowStatement {
  /**
   * SQL text for the current result, with bind placeholders left as written.
   * `undefined` when the statement came from `fetchResult()` or `getResultsFromQueryId()`
   * (no SQL was supplied).
   *
   * - Single statement: the string passed to `execute()`, including a trailing `;`.
   * - Multi-statement: the current `;`-separated segment. Advances with {@link RowStatement.NextResult}.
   */
  getSqlText(): string | undefined;

  /**
   * Returns the current status of this statement.
   */
  getStatus(): StatementStatus;

  /**
   * Number of rows in the current result.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getNumRows(): number | undefined;

  /**
   * Number of rows that an `INSERT`, `UPDATE`, `DELETE`, `MERGE` or `COPY INTO <table>`
   * statement changed or loaded. `-1` for other statements.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getNumUpdatedRows(): number | undefined;

  /**
   * Query ID that Snowflake gave to this statement.
   * `undefined` until the `complete` callback is called. After a failure, it is set when
   * Snowflake returned a query ID, and `undefined` when the request did not reach Snowflake.
   */
  getQueryId(): string | undefined;

  /**
   * Request ID sent to Snowflake with this query: the `requestId` option of `execute()`, or a
   * random UUID. It is set as soon as `execute()` returns.
   * `undefined` for statements created by `fetchResult()` or `getResultsFromQueryId()`,
   * because they read the result of an earlier query and send no request ID.
   */
  getRequestId(): string | undefined;

  /**
   * Current database, schema, warehouse and role of the session right after this query.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getSessionState(): SessionState | undefined;

  /**
   * Columns of the current result.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getColumns(): Column[] | undefined;

  /**
   * Column of the current result with the given name, or at the given 0-based index.
   * When several columns have the same name, it returns the first one.
   * `undefined` when no column matches, while the query is running, after it fails, and for
   * `asyncExec` queries.
   */
  getColumn(columnIdentifier: string | number): Column | undefined;

  /**
   * Returns true if a multi-statement query has more results.
   * {@link RowStatement.NextResult} moves to the next one.
   */
  hasNext(): boolean;

  /**
   * Moves to the next result of a multi-statement query.
   * The `complete` callback passed to `execute()` is called again with this statement.
   *
   * When {@link RowStatement.hasNext} is `false`, it does nothing: `complete`
   * is not called and the statement keeps its current result.
   *
   * Errors are passed to `complete`:
   * - The query failed: `complete` gets the same error again.
   * - Loading the next result from Snowflake failed: `complete` gets that error.
   */
  NextResult(): void;

  /**
   * Streams the rows in this statement's result.
   */
  streamRows(options?: StreamOptions): Readable;

  /**
   * Cancels this statement if possible.
   */
  cancel(callback?: StatementCallback): void;
}
