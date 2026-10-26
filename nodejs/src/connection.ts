import { randomUUID } from 'node:crypto';
import type { SnowflakeError } from './error.js';
import type { ConnectionOptions } from './public-types/index.js';
import type {
  StatementCallback,
  DataType,
  RowMode,
  RowOptions,
  QueryStatus,
} from './query-result/types.js';
import { toCoreConnectionOptions } from './connection-option-aliases.js';
import { serializeTokenInfo, deserializeTokenInfo } from './connection-serialization.js';
import {
  CoreConnection,
  CoreQueryBindingFormat,
  type CoreConnectionInstance,
  type CoreStatementInstance,
  type ConnectionTokenInfo,
} from './core/index.js';
import { GlobalConfig } from './global-config.js';
import { type Binds, buildBindsMap, selectBindPayload } from './query-result/binds.js';
import { collectRows } from './query-result/rows.js';
import { RowStatement } from './query-result/RowStatement.js';
import { getWrapperIdentity } from './wrapper-identity.js';

export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// Not exported, so session tokens reach the bridge only through deserializeConnection.
const TOKEN_INFO = Symbol('tokenInfo');
interface InternalConnectionOptions extends ConnectionOptions {
  [TOKEN_INFO]?: ConnectionTokenInfo;
}

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility
export interface StatementOption {
  sqlText: string;
  complete?: StatementCallback;
  asyncExec?: boolean;
  streamResult?: boolean;

  /**
   * Parameters scoped to this single statement, sent with the execute request
   * rather than applied to the whole session. Keys are Snowflake statement-level
   * parameter names (e.g. `TIME_OUTPUT_FORMAT`);
   */
  parameters?: Record<string, unknown>;

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

  /**
   * Current working directory to use for GET/PUT execution using relative paths from a client location
   * that is different from the connector directory.
   */
  cwd?: string;

  /**
   * UUID used to resubmit a request. When omitted, the driver creates one.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-execute
   */
  requestId?: string;

  /**
   * When `true`, the query is submitted for column metadata only.
   */
  describeOnly?: boolean;
}

export interface FetchResultOptions {
  queryId: string;
  complete?: StatementCallback;
  streamResult?: boolean;
  rowMode?: RowMode;
  fetchAsString?: DataType[];
}

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
export class Connection {
  #core: CoreConnectionInstance;
  #defaultRowOptions: RowOptions;
  #id: string;

  constructor(options?: ConnectionOptions) {
    const callerOptions = options ?? {};
    const {
      rowMode,
      fetchAsString,
      jsTreatIntegerAsBigInt,
      representNullAsStringNull,
      rowStreamHighWaterMark,
      arrayBindingThreshold,
      openExternalBrowserCallback,
      [TOKEN_INFO]: tokenInfo,
      ...coreOptions
    } = callerOptions as InternalConnectionOptions;
    const noConnectionDetails =
      tokenInfo === undefined && Object.values(callerOptions).every((value) => value === undefined);
    this.#id = randomUUID();

    this.#defaultRowOptions = {
      rowMode: rowMode ?? 'object',
      fetchAsString: fetchAsString ?? [],
      representNullAsStringNull: representNullAsStringNull ?? true,
      rowStreamHighWaterMark: rowStreamHighWaterMark ?? 10,
    };

    const sessionParameters: Record<string, string> = {};
    if (jsTreatIntegerAsBigInt !== undefined) {
      sessionParameters['JS_TREAT_INTEGER_AS_BIGINT'] = String(jsTreatIntegerAsBigInt);
    }
    if (arrayBindingThreshold !== undefined) {
      sessionParameters['CLIENT_STAGE_ARRAY_BINDING_THRESHOLD'] = String(arrayBindingThreshold);
    }

    this.#core = new CoreConnection({
      options: toCoreConnectionOptions({
        crlDownloadTimeout: GlobalConfig.crlDownloadTimeout,
        crlDownloadMaxSize: GlobalConfig.crlDownloadMaxSize,
        crlCacheValidityTime: GlobalConfig.crlCacheValidityTime,
        crlCacheDir: GlobalConfig.crlCacheDir,
        ...coreOptions,
        useEnvProxy: GlobalConfig.useEnvProxy,
      }),
      sessionParameters,
      openExternalBrowserCallback,
      tokenInfo,
      noConnectionDetails,
      wrapperIdentity: getWrapperIdentity(),
    });
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

  serialize(): string {
    return serializeTokenInfo(this.#core.getTokenInfo());
  }

  isUp(): boolean {
    return this.#core.isUp();
  }

  getId(): string {
    return this.#id;
  }

  /**
   * Sends a heartbeat to the server, which also renews the session. Resolves `false` when the
   * session is gone rather than rejecting.
   */
  isValidAsync(): Promise<boolean> {
    return this.#core.isValidAsync();
  }

  execute(options: StatementOption): RowStatement {
    const { binds } = options;
    const bindings = selectBindPayload(
      binds,
      this.#core.getSessionParameters().clientStageArrayBindingThreshold,
    );
    // Binds are serialized here in JS, while the retry with inline binds after a failed stage
    // upload runs in the bridge, so the bridge calls back for the JSON payload. The intended end
    // state is that `#core.execute` takes the raw binds and the bridge builds the CSV or JSON
    // payload itself; this callback goes away then.
    const buildFallbackInlineBindings =
      binds && bindings?.format === CoreQueryBindingFormat.Csv
        ? () => JSON.stringify(buildBindsMap(binds))
        : undefined;

    let parameters = options.parameters
      ? Object.fromEntries(
          Object.entries(options.parameters).map(([key, value]) => [key, String(value)]),
        )
      : null;
    if (options.cwd) {
      parameters = { ...parameters, cwd: options.cwd };
    }

    return this.#runStatement(
      this.#core.execute(
        {
          query: options.sqlText,
          bindings: bindings ?? undefined,
          parameters: parameters ?? undefined,
          asyncExec: options.asyncExec,
          requestId: options.requestId,
          describeOnly: options.describeOnly,
        },
        buildFallbackInlineBindings,
      ),
      {
        complete: options.complete,
        streamResult: options.streamResult,
        sqlText: options.sqlText,
        asyncExec: options.asyncExec,
        rowOptions: {
          ...this.#defaultRowOptions,
          ...(options.rowMode && { rowMode: options.rowMode }),
          ...(options.fetchAsString && { fetchAsString: options.fetchAsString }),
        },
      },
    );
  }

  async getQueryStatus(queryId: string): Promise<QueryStatus> {
    return this.#core.getQueryStatus(queryId);
  }

  async getQueryStatusThrowIfError(queryId: string): Promise<QueryStatus> {
    return this.#core.getQueryStatusThrowIfError(queryId);
  }

  isStillRunning(status: QueryStatus): boolean {
    return this.#core.isStillRunning(status);
  }

  isAnError(status: QueryStatus): boolean {
    return this.#core.isAnError(status);
  }

  fetchResult(options: FetchResultOptions): RowStatement {
    return this.#runStatement(this.#core.getQueryResult(options.queryId), {
      complete: options.complete,
      streamResult: options.streamResult,
      rowOptions: {
        ...this.#defaultRowOptions,
        ...(options.rowMode && { rowMode: options.rowMode }),
        ...(options.fetchAsString && { fetchAsString: options.fetchAsString }),
      },
    });
  }

  async getResultsFromQueryId(options: FetchResultOptions): Promise<RowStatement> {
    const retryIntervalMs = (options as { _testOnlyRetryIntervalMs?: number })
      ._testOnlyRetryIntervalMs;
    await this.#core.waitForQueryResult(options.queryId, retryIntervalMs);
    return this.fetchResult(options);
  }

  #runStatement(
    coreStatement: CoreStatementInstance,
    options: {
      complete?: StatementCallback;
      streamResult?: boolean;
      sqlText?: string;
      asyncExec?: boolean;
      rowOptions: RowOptions;
    },
  ): RowStatement {
    const { complete, streamResult, sqlText, asyncExec, rowOptions } = options;
    const statement = new RowStatement(coreStatement, rowOptions, sqlText, complete);
    (async () => {
      try {
        await coreStatement.waitForCompletion();
        if (asyncExec === true || streamResult === true || coreStatement.hasNext()) {
          complete?.(undefined, statement, undefined);
        } else {
          complete?.(undefined, statement, await collectRows(coreStatement, rowOptions));
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

export const createConnection = (options?: ConnectionOptions) => new Connection(options);
export const serializeConnection = (connection: Connection): string => connection.serialize();
/**
 * Rebuilds a connection from a string produced by {@link Connection.serialize}.
 * The new connection addresses the same server session as the connection that
 * produced the string. `destroy()` on either connection logs that session out.
 * A payload without both tokens throws, because it has no session to restore.
 */
export const deserializeConnection = (
  options: ConnectionOptions,
  serializedConnection: string,
): Connection => {
  const internalOptions: InternalConnectionOptions = {
    ...options,
    [TOKEN_INFO]: deserializeTokenInfo(serializedConnection),
  };
  return new Connection(internalOptions);
};
