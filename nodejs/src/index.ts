import { CoreConnection, type CoreConnectionInstance } from './core';
import { RowStatement, FileAndStageBindStatement } from './query-result/RowStatement.js';

// TODO: implement SnowflakeError like in old driver
export type SnowflakeError = Error;
// TODO: implement ConnectionOptions like in old driver
export type ConnectionOptions = Record<string, string>;
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
}

export type StatementCallback = (
  err: SnowflakeError | undefined,
  stmt: RowStatement | FileAndStageBindStatement,
  rows: Array<unknown> | undefined,
) => void;

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
class Connection {
  #core: CoreConnectionInstance;

  constructor(options: ConnectionOptions) {
    this.#core = new CoreConnection(options);
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

  // TODO:
  // Consider a BCR where execute will not return unusable RowStatement and will be only
  // available inside a callback
  execute(options: StatementOption): RowStatement | FileAndStageBindStatement {
    const executePromise = this.#core.execute(options.sqlText);
    const rowStatement = new RowStatement(executePromise);

    // TODO:
    // This block looks ugly, it will evolve into something better as we implement all properties
    // from the old driver's StatementOption interface
    executePromise
      .then(async (coreStatement) => {
        try {
          const rows: unknown[] = [];
          while (true) {
            const row = await coreStatement.getNextRow();
            if (row === null) {
              break;
            }
            rows.push(row);
          }
          options.complete?.(undefined, rowStatement, rows);
        } finally {
          coreStatement.close();
        }
      })
      .catch((err: Error) => {
        options.complete?.(err as SnowflakeError, rowStatement, undefined);
      });

    return rowStatement;
  }
}

export const createConnection = (options: ConnectionOptions) => {
  return new Connection(options);
};
