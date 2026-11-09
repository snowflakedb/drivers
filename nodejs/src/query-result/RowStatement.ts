import type { Readable } from 'node:stream';
import type { CoreConnectionInstance, CoreStatementInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type { Column, RowStatement } from '../public-types/index.js';
import type {
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
export class RowStatementImpl implements RowStatement {
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

  getSqlText(): string | undefined {
    this.#checkMultiSqlTexts();
    return this.#multiSqlTexts ? this.#multiSqlTexts[this.#currentSqlIndex] : this.#sqlText;
  }

  getStatus(): StatementStatus {
    return this.#core.getStatus();
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
    return (
      this.#core.getColumns(shouldRenameDuplicateColumns(this.#rowOptions.rowMode)) ?? undefined
    );
  }

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
