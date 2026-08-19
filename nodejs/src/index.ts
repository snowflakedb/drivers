import type { SnowflakeError } from './error.js';
import { normalizeConnectionOptions } from './connection-option-aliases.js';
import { CoreConnection, type CoreConnectionInstance, type CoreStatementInstance } from './core';
import {
  updateGlobalConfig,
  type ConfigureOptions,
  type CustomParser,
  type XMlParserConfigOption,
} from './global-config.js';
import { collectRows } from './query-result/rows.js';
import {
  RowStatement,
  FileAndStageBindStatement,
  type StatementCallback,
  type StreamOptions,
  type DataType,
  type FetchRowsOptions,
  type Column,
} from './query-result/RowStatement.js';

export {
  RowStatement,
  type StatementCallback,
  type StreamOptions,
  type DataType,
  type FetchRowsOptions,
  type Column,
  type CustomParser,
  type XMlParserConfigOption,
  type ConfigureOptions,
};

// TODO: implement ConnectionOptions like in old driver
export type ConnectionOptions = Record<string, string>;
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
  streamResult?: boolean;
}

export interface FetchResultOptions {
  queryId: string;
  complete?: StatementCallback;
  streamResult?: boolean;
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

  execute(options: StatementOption): RowStatement | FileAndStageBindStatement {
    return this.#runStatement(
      this.#core.execute(options.sqlText),
      options.complete,
      options.streamResult,
    );
  }

  fetchResult(options: FetchResultOptions): RowStatement | FileAndStageBindStatement {
    return this.#runStatement(
      this.#core.getQueryResult(options.queryId),
      options.complete,
      options.streamResult,
    );
  }

  #runStatement(
    coreStatement: CoreStatementInstance,
    complete?: StatementCallback,
    streamResult?: boolean,
  ): RowStatement | FileAndStageBindStatement {
    const statement = new RowStatement(coreStatement);
    (async () => {
      try {
        if (streamResult === true) {
          await coreStatement.wait();
          complete?.(undefined, statement, undefined);
        } else {
          complete?.(undefined, statement, await collectRows(coreStatement));
        }
      } catch (err) {
        complete?.(err as SnowflakeError, statement, undefined);
      }
    })();
    return statement;
  }

  destroy(callback?: ConnectionCallback) {
    this.#core
      .destroy()
      .then(() => callback?.(undefined, this))
      .catch((err) => callback?.(err, this));
  }
}

// TODO: JSDoc needed
export const configure = (options: ConfigureOptions) => updateGlobalConfig(options);
export const createConnection = (options: ConnectionOptions) => new Connection(options);
