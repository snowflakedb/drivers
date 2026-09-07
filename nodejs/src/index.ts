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
import {
  CoreConnection,
  CoreQueryBindings,
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
import {
  type Binds,
  type InsertBinds,
  type Bind,
  selectBindPayload,
} from './query-result/binds.js';
import { collectRows } from './query-result/rows.js';
import { RowStatement, FileAndStageBindStatement } from './query-result/RowStatement.js';

// TODO:
// consider exporting directly from files so its easier to understand where the type comes from
export {
  type Bind,
  type Binds,
  type InsertBinds,
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
  /**
   * Decides when large bulk {@link StatementOption.binds} are uploaded to a temporary stage instead
   * of being sent with the request.
   *
   * The driver counts the total number of bound values (rows * columns). If the count is greater
   * than this number, the driver uploads the values to a stage as a CSV file, and the query reads
   * them from that file. If the count is this number or lower, the values are sent inline in the
   * request. Uploading is faster for big batches; inline is simpler for small ones. Single-row
   * binds are always sent inline and ignore this option.
   *
   * When set, the connection starts with this value as its custom
   * `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` session parameter.
   *
   * @default User's CLIENT_STAGE_ARRAY_BINDING_THRESHOLD value
   */
  arrayBindingThreshold?: number;
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
  /**
   * Values for the placeholders in {@link StatementOption.sqlText}. Write `?` (or
   * `:1`, `:2`, ...) in the SQL, and list the values here in the same order. The
   * driver sends them as bind parameters, so they are safe from SQL injection.
   *
   * Pass one row of values to run the statement once:
   *
   * @example
   * connection.execute({
   *   sqlText: 'SELECT c1 FROM t WHERE c1 = ?',
   *   binds: [1],
   * });
   *
   * Pass an array of rows to run a bulk `INSERT` (one row per inner array):
   *
   * @example
   * connection.execute({
   *   sqlText: 'INSERT INTO t(c1, c2, c3) VALUES(?, ?, ?)',
   *   binds: [[1, 'string1', 2.0], [2, 'string2', 4.0]],
   * });
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-execute
   */
  binds?: Binds;
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
      arrayBindingThreshold,
      ...coreOptions
    } = options;

    this.#defaultRowOptions = {
      rowMode: rowMode ?? 'object',
      fetchAsString: fetchAsString ?? [],
      representNullAsStringNull: representNullAsStringNull ?? true,
    };

    const sessionParameters: Record<string, string> = {};
    if (jsTreatIntegerAsBigInt !== undefined) {
      sessionParameters['JS_TREAT_INTEGER_AS_BIGINT'] = String(jsTreatIntegerAsBigInt);
    }
    if (arrayBindingThreshold !== undefined) {
      sessionParameters['CLIENT_STAGE_ARRAY_BINDING_THRESHOLD'] = String(arrayBindingThreshold);
    }

    this.#core = new CoreConnection(
      // Cast until options are typed across the bridge, which takes strings only.
      normalizeConnectionOptions({
        ...(coreOptions as Record<string, string>),
        useEnvProxy: String(GlobalConfig.useEnvProxy),
      }),
      sessionParameters,
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

  /**
   * Sends a heartbeat to the server, which also renews the session. Resolves `false` when the
   * session is gone rather than rejecting.
   */
  isValidAsync(): Promise<boolean> {
    return this.#core.isValidAsync();
  }

  execute(options: StatementOption): RowStatement | FileAndStageBindStatement {
    let bindings: CoreQueryBindings | null = null;

    // getSessionParameters() throws a different error when the connection is not up.
    // Swallowing it leaves bindings null so callers see core.execute()'s
    // connection-state error via #runStatement. One of two fixes would remove the swallow:
    // - Gate on isUp() and pass null bindings when false. isUp() is not implemented yet.
    // - Move bind selection into the bridge, which already gates on connection state
    //   (preferred: the session-parameter read leaves this layer).
    try {
      const { clientStageArrayBindingThreshold } = this.#core.getSessionParameters();
      bindings = selectBindPayload(options.binds, clientStageArrayBindingThreshold);
    } catch {
      // Ignore the error
    }

    return this.#runStatement(this.#core.execute(options.sqlText, bindings), {
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
