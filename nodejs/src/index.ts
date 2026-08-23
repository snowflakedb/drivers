import type { SnowflakeError } from './error.js';
import type {
  StatementCallback,
  StreamOptions,
  DataType,
  FetchRowsOptions,
  Column,
  RowMode,
} from './query-result/types.js';
import { normalizeConnectionOptions } from './connection-option-aliases.js';
import { CoreConnection, type CoreConnectionInstance, type CoreStatementInstance } from './core';
import {
  updateGlobalConfig,
  type ConfigureOptions,
  type CustomParser,
  type XMlParserConfigOption,
} from './global-config.js';
import { collectRows } from './query-result/rows.js';
import { RowStatement, FileAndStageBindStatement } from './query-result/RowStatement.js';

export {
  RowStatement,
  type StatementCallback,
  type StreamOptions,
  type DataType,
  type FetchRowsOptions,
  type Column,
  type RowMode,
  type CustomParser,
  type XMlParserConfigOption,
  type ConfigureOptions,
};

// TODO: implement ConnectionOptions like in old driver (BD#2)
export type ConnectionOptions = Record<string, string> & { rowMode?: RowMode };
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
  streamResult?: boolean;
  rowMode?: RowMode;
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
  #defaultRowMode?: RowMode;

  constructor(options: ConnectionOptions) {
    const { rowMode, ...coreOptions } = options;
    this.#defaultRowMode = rowMode;
    this.#core = new CoreConnection(normalizeConnectionOptions(coreOptions));
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
    return this.#runStatement(this.#core.execute(options.sqlText), {
      complete: options.complete,
      streamResult: options.streamResult,
      rowMode: options.rowMode ?? this.#defaultRowMode,
    });
  }

  fetchResult(options: FetchResultOptions): RowStatement | FileAndStageBindStatement {
    return this.#runStatement(this.#core.getQueryResult(options.queryId), {
      complete: options.complete,
      streamResult: options.streamResult,
      // The old driver's fetchResult() never fell back to the connection-level
      // rowMode default, unlike execute() -- that asymmetry is treated as a bug
      // here, not preserved (BD#3).
      rowMode: this.#defaultRowMode,
    });
  }

  // TODO: options are drilled 3 levels deep here -- execute/fetchResult ->
  // #runStatement -> RowStatement/collectRows -- threading each StatementOption
  // field by hand; revisit once config handling is fully implemented (see the
  // ConnectionOptions TODO above, BD#2).
  #runStatement(
    coreStatement: CoreStatementInstance,
    options: {
      complete?: StatementCallback;
      streamResult?: boolean;
      rowMode?: RowMode;
    },
  ): RowStatement | FileAndStageBindStatement {
    const { complete, streamResult, rowMode } = options;
    const statement = new RowStatement(coreStatement, rowMode);
    (async () => {
      try {
        if (streamResult === true) {
          await coreStatement.waitForCompletion();
          complete?.(undefined, statement, undefined);
        } else {
          complete?.(undefined, statement, await collectRows(coreStatement, statement.rowMode));
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
