import type { Readable } from 'node:stream';
import type { CoreConnectionInstance, CoreStatementInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type {
  Column,
  RowOptions,
  SessionState,
  StatementCallback,
  StatementStatus,
  StreamOptions,
} from './types.js';
import { createRowStream } from './rows.js';
import { shouldRenameDuplicateColumns } from './types.js';

// TODO:
// Not sure if we actually need to have a wrapper around core statement.
// My initial thought is that we might not be able to implement all public
// driver methods from Rust or that using so many FFI calls won't be efficient.
// Refactor if that won't be the case.
export class RowStatement {
  #connection: CoreConnectionInstance;
  #core: CoreStatementInstance;
  #rowOptions: RowOptions;
  #sqlText?: string;
  // TODO: consider removing, split on ';' can produce false positives, kept for backward compatibility
  #multiSqlTexts?: string[];
  #currentSqlIndex = 0;
  #complete?: StatementCallback;

  constructor(
    connection: CoreConnectionInstance,
    core: CoreStatementInstance,
    rowOptions: RowOptions,
    sqlText?: string,
    complete?: StatementCallback,
  ) {
    this.#connection = connection;
    this.#core = core;
    this.#rowOptions = rowOptions;
    this.#sqlText = sqlText;
    this.#complete = complete;
  }

  /**
   * SQL text for the current result, with bind placeholders left as written.
   * `undefined` when the statement came from `fetchResult()` or `getResultsFromQueryId()`
   * (no SQL was supplied).
   *
   * - Single statement: the string passed to `execute()`, including a trailing `;`.
   * - Multi-statement: the current `;`-separated segment. Advances with {@link NextResult}.
   */
  getSqlText(): string | undefined {
    this.#checkMultiSqlTexts();
    return this.#multiSqlTexts ? this.#multiSqlTexts[this.#currentSqlIndex] : this.#sqlText;
  }

  getStatus(): StatementStatus {
    return this.#core.getStatus();
  }

  /**
   * Number of rows in the current result.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getNumRows(): number | undefined {
    return this.#core.getNumRows() ?? undefined;
  }

  /**
   * Number of rows that an `INSERT`, `UPDATE`, `DELETE`, `MERGE` or `COPY INTO <table>`
   * statement changed or loaded. `-1` for other statements.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getNumUpdatedRows(): number | undefined {
    return this.#core.getNumUpdatedRows() ?? undefined;
  }

  /**
   * Query ID that Snowflake gave to this statement.
   * `undefined` until the `complete` callback is called. After a failure, it is set when
   * Snowflake returned a query ID, and `undefined` when the request did not reach Snowflake.
   */
  getQueryId(): string | undefined {
    return this.#core.getQueryId() ?? undefined;
  }

  /**
   * Request ID sent to Snowflake with this query: the `requestId` option of `execute()`, or a
   * random UUID. It is set as soon as `execute()` returns.
   * `undefined` for statements created by `fetchResult()` or `getResultsFromQueryId()`,
   * because they read the result of an earlier query and send no request ID.
   */
  getRequestId(): string | undefined {
    return this.#core.getRequestId() ?? undefined;
  }

  /**
   * Current database, schema, warehouse and role of the session right after this query.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getSessionState(): SessionState | undefined {
    return this.#core.getSessionState() ?? undefined;
  }

  /**
   * Columns of the current result.
   * `undefined` while the query is running, after it fails, and for `asyncExec` queries.
   */
  getColumns(): Column[] | undefined {
    return (
      this.#core.getColumns(shouldRenameDuplicateColumns(this.#rowOptions.rowMode)) ?? undefined
    );
  }

  /**
   * Column of the current result with the given name, or at the given 0-based index.
   * When several columns have the same name, it returns the first one.
   * `undefined` when no column matches, while the query is running, after it fails, and for
   * `asyncExec` queries.
   */
  getColumn(columnIdentifier: string | number): Column | undefined {
    return (
      this.#core.getColumn(
        columnIdentifier,
        shouldRenameDuplicateColumns(this.#rowOptions.rowMode),
      ) ?? undefined
    );
  }

  hasNext(): boolean {
    return this.#core.hasNext();
  }

  NextResult(): void {
    void this.#core
      .nextResult()
      .then((advanced) => {
        if (!advanced) {
          return;
        }
        this.#currentSqlIndex += 1;
        this.#complete?.(undefined, this, undefined);
      })
      .catch((err: Error) => {
        this.#complete?.(err as SnowflakeError, this, undefined);
      });
  }

  // TODO: decide how to handle a case where user didn't set the streamResult: true
  // and the result is already drained. (would suggest a BCR with error)
  // oxlint-disable-next-line no-unused-vars
  streamRows(options?: StreamOptions): Readable {
    return createRowStream(this.#connection, this.#core, {
      ...this.#rowOptions,
      fetchAsString: options?.fetchAsString ?? this.#rowOptions.fetchAsString,
    });
  }

  #checkMultiSqlTexts(): void {
    if (
      this.#core.isMultiStatement() &&
      this.#multiSqlTexts === undefined &&
      this.#sqlText !== undefined
    ) {
      this.#multiSqlTexts = this.#sqlText.split(';');
    }
  }

  cancel(callback?: StatementCallback): void {
    Promise.resolve()
      .then(() => this.#core.cancel())
      .then(() => callback?.(undefined, this, undefined))
      .catch((err: Error) => callback?.(err as SnowflakeError, this, undefined));
  }
}
