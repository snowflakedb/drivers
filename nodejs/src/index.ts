import type { SnowflakeError } from './error.js';
import { normalizeConnectionOptions } from './connection-option-aliases.js';
import { CoreConnection, type CoreConnectionInstance } from './core';
import { collectRows } from './query-result/rows.js';
import {
  RowStatement,
  FileAndStageBindStatement,
  type StatementCallback,
} from './query-result/RowStatement.js';

export { RowStatement, type StatementCallback };

// TODO: implement ConnectionOptions like in old driver
export type ConnectionOptions = Record<string, string>;
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
}

export interface FetchResultOptions {
  queryId: string;
  complete?: StatementCallback;
}

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
export class Connection {
  #core: CoreConnectionInstance;

  constructor(options: ConnectionOptions) {
    this.#core = new CoreConnection(normalizeConnectionOptions(options));
  }

  connect(callback?: ConnectionCallback) {
    this.connectAsync()
      .then(() => {
        callback?.(undefined, this);
      })
      .catch((err) => {
        callback?.(err, this);
      });
  }

  connectAsync(): Promise<void> {
    return this.#core.connect();
  }

  // TODO: BCR for execute and fetchResult returning an unusable RowStatement
  // - Applies to both execute and fetchResult
  // - Option A: do not return RowStatement synchronously — only available in complete callback
  // - Option B: throw on RowStatement method calls if used before the statement is ready
  // - Related: old driver returns undefined from getNumRows()/getQueryId() when called
  //   before query completion or when the query errors / returns no rows (see BCR_LOG.md)
  execute(options: StatementOption): RowStatement | FileAndStageBindStatement {
    const executePromise = this.#core.execute(options.sqlText);
    const rowStatement = new RowStatement(executePromise);

    // TODO:
    // This block looks ugly, it will evolve into something better as we implement all properties
    // from the old driver's StatementOption interface
    executePromise
      .then(async (coreStatement) => {
        const rows = await collectRows(coreStatement);
        options.complete?.(undefined, rowStatement, rows);
      })
      .catch((err: Error) => {
        options.complete?.(err as SnowflakeError, rowStatement, undefined);
      });

    return rowStatement;
  }

  fetchResult(options: FetchResultOptions): RowStatement | FileAndStageBindStatement {
    const queryResultPromise = this.#core.getQueryResult(options.queryId);
    const rowStatement = new RowStatement(queryResultPromise);

    queryResultPromise
      .then(async (coreStatement) => {
        const rows = await collectRows(coreStatement);
        options.complete?.(undefined, rowStatement, rows);
      })
      .catch((err: Error) => {
        options.complete?.(err as SnowflakeError, rowStatement, undefined);
      });

    return rowStatement;
  }

  destroy(callback?: ConnectionCallback) {
    this.#core
      .destroy()
      .then(() => callback?.(undefined, this))
      .catch((err) => callback?.(err, this));
  }
}

export const createConnection = (options: ConnectionOptions) => {
  return new Connection(options);
};
