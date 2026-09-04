import type { SnowflakeError } from './error.js';
import type {
  StatementCallback,
  StreamOptions,
  DataType,
  FetchRowsOptions,
  Column,
  RowMode,
  RowOptions,
} from './query-result/types.js';
import { normalizeConnectionOptions } from './connection-option-aliases.js';
import ErrorCode from './constants/ErrorCode.js';
import { OcspMode as ocspModes } from './constants/OcspMode.js';
import SessionParameterName from './constants/SessionParameterName.js';
import {
  CoreConnection,
  type CoreConnectionInstance,
  type CoreStatementInstance,
} from './core/index.js';
import {
  GlobalConfig,
  updateGlobalConfig,
  type ConfigureOptions,
  type CustomParser,
  type XMlParserConfigOption,
} from './global-config.js';
import { collectRows } from './query-result/rows.js';
import { RowStatement, FileAndStageBindStatement } from './query-result/RowStatement.js';

export {
  type RowStatement,
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
export type ConnectionOptions = Record<string, unknown> & {
  rowMode?: RowMode;
  fetchAsString?: DataType[];
  jsTreatIntegerAsBigInt?: boolean;
  /**
   * Controls how a SQL NULL is rendered for columns that {@link fetchAsString}
   * returns as strings. A column returned as its native JavaScript type is
   * unaffected — its NULL is always `null`.
   *
   * When `true`, a NULL in a string-rendered column comes back as the string
   * `'NULL'`; when `false`, it stays `null`.
   *
   * TODO(BD#18): some data types do not route their NULL through this option yet
   * (see `nodejs/BehaviorDifferences.yaml`). Document the per-type behavior here
   * once every data type is implemented.
   *
   * @default true
   */
  representNullAsStringNull?: boolean;
};
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
  streamResult?: boolean;
  rowMode?: RowMode;
  fetchAsString?: DataType[];
}

export interface FetchResultOptions {
  queryId: string;
  complete?: StatementCallback;
  streamResult?: boolean;
  fetchAsString?: DataType[];
}

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
export class Connection {
  #core: CoreConnectionInstance;
  #defaultRowOptions: RowOptions;

  constructor(options: ConnectionOptions) {
    const {
      rowMode,
      fetchAsString,
      jsTreatIntegerAsBigInt,
      representNullAsStringNull,
      ...coreOptions
    } = options;
    this.#defaultRowOptions = {
      rowMode: rowMode ?? 'object',
      fetchAsString: fetchAsString ?? [],
      representNullAsStringNull: representNullAsStringNull ?? true,
    };
    this.#core = new CoreConnection(
      // Cast until options are typed across the bridge, which takes strings only.
      normalizeConnectionOptions({
        ...(coreOptions as Record<string, string>),
        useEnvProxy: String(GlobalConfig.useEnvProxy),
      }),
      jsTreatIntegerAsBigInt === undefined
        ? {}
        : { [SessionParameterName.JS_TREAT_INTEGER_AS_BIGINT]: String(jsTreatIntegerAsBigInt) },
    );
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

  isUp(): boolean {
    return this.#core.isUp();
  }

  execute(options: StatementOption): RowStatement | FileAndStageBindStatement {
    return this.#runStatement(this.#core.execute(options.sqlText), {
      complete: options.complete,
      streamResult: options.streamResult,
      rowOptions: {
        ...this.#defaultRowOptions,
        ...(options.rowMode && { rowMode: options.rowMode }),
        ...(options.fetchAsString && { fetchAsString: options.fetchAsString }),
      },
    });
  }

  fetchResult(options: FetchResultOptions): RowStatement | FileAndStageBindStatement {
    return this.#runStatement(this.#core.getQueryResult(options.queryId), {
      complete: options.complete,
      streamResult: options.streamResult,
      rowOptions: {
        // The old driver's fetchResult() never fell back to the connection-level
        // rowMode default, unlike execute() -- that asymmetry is treated as a bug
        // here, not preserved (BD#3).
        ...this.#defaultRowOptions,
        ...(options.fetchAsString && { fetchAsString: options.fetchAsString }),
      },
    });
  }

  #runStatement(
    coreStatement: CoreStatementInstance,
    options: {
      complete?: StatementCallback;
      streamResult?: boolean;
      rowOptions: RowOptions;
    },
  ): RowStatement | FileAndStageBindStatement {
    const { complete, streamResult, rowOptions } = options;
    const statement = new RowStatement(this.#core, coreStatement, rowOptions);
    (async () => {
      try {
        if (streamResult === true) {
          await coreStatement.waitForCompletion();
          complete?.(undefined, statement, undefined);
        } else {
          complete?.(
            undefined,
            statement,
            await collectRows(this.#core, coreStatement, rowOptions),
          );
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

// TODO:
// - JSDoc needed
// - Map to similar shape as old driver where we have core object that has bunch of methods and
//   it is exported as default
export const configure = (options: ConfigureOptions) => updateGlobalConfig(options);
export const createConnection = (options: ConnectionOptions) => new Connection(options);

export default {
  configure,
  createConnection,
  ErrorCode,
  ocspModes,
};
