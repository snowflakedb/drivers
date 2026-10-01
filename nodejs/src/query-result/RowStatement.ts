import type { Readable } from 'node:stream';
import type { CoreStatementInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type {
  Column,
  FetchRowsOptions,
  RowOptions,
  SessionState,
  StatementCallback,
  StreamOptions,
} from './types.js';
import { createRowStream } from './rows.js';

// TODO:
// Not sure if we actually need to have a wrapper around core statement.
// My initial thought is that we might not be able to implement all public
// driver methods from Rust or that using so many FFI calls won't be efficient.
// Refactor if that won't be the case.
export class RowStatement {
  #core: CoreStatementInstance;
  #rowOptions: RowOptions;
  #sqlText?: string;
  // TODO: consider removing, split on ';' can produce false positives, kept for backward compatibility
  #multiSqlTexts?: string[];
  #currentSqlIndex = 0;
  #complete?: StatementCallback;

  constructor(
    core: CoreStatementInstance,
    rowOptions: RowOptions,
    sqlText?: string,
    complete?: StatementCallback,
  ) {
    this.#core = core;
    this.#rowOptions = rowOptions;
    this.#sqlText = sqlText;
    this.#complete = complete;
  }

  /**
   * SQL text for the current result, with bind placeholders left as written.
   * `undefined` when the statement came from `fetchResult()` (no SQL was supplied).
   *
   * - Single statement: the string passed to `execute()`, including a trailing `;`.
   * - Multi-statement: the current `;`-separated segment. Advances with {@link NextResult}.
   */
  getSqlText(): string | undefined {
    this.#checkMultiSqlTexts();
    return this.#multiSqlTexts ? this.#multiSqlTexts[this.#currentSqlIndex] : this.#sqlText;
  }

  getNumRows(): number | undefined {
    return this.#core.getNumRows() ?? undefined;
  }

  getNumUpdatedRows(): number | undefined {
    return this.#core.getNumUpdatedRows() ?? undefined;
  }

  getQueryId(): string | undefined {
    return this.#core.getQueryId() ?? undefined;
  }

  getRequestId(): string | undefined {
    return this.#core.getRequestId() ?? undefined;
  }

  getSessionState(): SessionState | undefined {
    return this.#core.getSessionState() ?? undefined;
  }

  getColumns(): Column[] | undefined {
    return this.#core.getColumns() ?? undefined;
  }

  getColumn(columnIdentifier: string | number): Column | undefined {
    return this.#core.getColumn(columnIdentifier) ?? undefined;
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
    return createRowStream(this.#core, {
      ...this.#rowOptions,
      fetchAsString: options?.fetchAsString ?? this.#rowOptions.fetchAsString,
    });
  }

  fetchRows(options: FetchRowsOptions): void {
    const stream = createRowStream(this.#core, this.#rowOptions);
    let finished = false;

    const onComplete = (err: SnowflakeError | undefined) => {
      if (finished) {
        return;
      }
      finished = true;
      options.end(err, this);
    };

    stream.on('data', (row: unknown) => {
      if (finished) {
        return;
      }
      if (options.each(row) === false) {
        stream.destroy();
        onComplete(undefined);
      }
    });
    stream.on('end', () => onComplete(undefined));
    stream.on('error', (err: Error) => onComplete(err as SnowflakeError));
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
    this.#core
      .cancel()
      .then(() => callback?.(undefined, this, undefined))
      .catch((err: Error) => callback?.(err as SnowflakeError, this, undefined));
  }
}
