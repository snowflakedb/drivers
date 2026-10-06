import { randomUUID } from 'node:crypto';
import type { SnowflakeError } from './error.js';
import type {
  StatementCallback,
  StreamOptions,
  DataType,
  FetchRowsOptions,
  Column,
  RowMode,
  RowOptions,
  QueryStatus,
  SessionState,
  StatementStatus,
} from './query-result/types.js';
import { normalizeConnectionOptions } from './connection-option-aliases.js';
import { serializeTokenInfo, deserializeTokenInfo } from './connection-serialization.js';
import ErrorCode from './constants/ErrorCode.js';
import { OcspMode as ocspModes } from './constants/OcspMode.js';
import {
  CoreConnection,
  coreGetTlsStatus,
  coreIsAnError,
  coreIsStillRunning,
  initTypeConstructors,
  type CoreConnectionInstance,
  type CoreStatementInstance,
  type CoreTlsStatus,
} from './core/index.js';
import { createPool } from './create-pool.js';
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
import { RowStatement } from './query-result/RowStatement.js';
import { SnowflakeDate } from './query-result/SnowflakeDate.js';

initTypeConstructors({ SnowflakeDate });

export { createPool };
export type { Pool, PoolOptions } from './create-pool.js';

// TODO:
// consider exporting directly from files so its easier to understand where the type comes from
export {
  type SnowflakeError,
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
  type QueryStatus,
  SnowflakeDate,
  type SessionState,
  type StatementStatus,
};

/** Describes the linked Rustls TLS provider and build flag, not artifact compliance. */
export type TlsStatus = CoreTlsStatus;

// TODO: implement ConnectionOptions like in old driver (BD#2)
export type ConnectionOptions = Record<string, unknown> & {
  workloadIdentityProvider?: 'AWS' | 'AZURE' | 'GCP' | 'OIDC';
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
  /**
   * Optional string that can be used to tag queries and other SQL statements executed within a
   * connection. The tags are displayed in the output of the QUERY_HISTORY, QUERY_HISTORY_BY_*
   * functions.
   */
  queryTag?: string;
  /**
   * By default, client connections typically time out approximately 3-4 hours after the most recent query was executed.
   *
   * @default false
   */
  clientSessionKeepAlive?: boolean;
  /**
   * Sets the frequency (interval in seconds) between heartbeat messages.
   */
  clientSessionKeepAliveHeartbeatFrequency?: number;
  /**
   * When true, the session is not destroyed on the server side when the connection
   * is closed. This allows async queries to continue running after disconnect.
   * Any unfinished queries will continue to live in Snowflake and consume credits
   * until they finish.
   *
   * @default false
   */
  serverSessionKeepAlive?: boolean;
  /**
   * Replaces the system-browser launch used by `EXTERNALBROWSER` SSO and
   * `OAUTH_AUTHORIZATION_CODE`. The driver still binds the loopback
   * listener and waits for the IdP redirect; this function only receives
   * the URL that would otherwise be opened. The callback must return or
   * throw; a function that never returns holds a driver thread until
   * process exit, and login timeout does not unblock it. Login waits for
   * that synchronous return or throw. A returned Promise is not awaited
   * (an `async` callback that throws becomes a rejected Promise and is not
   * observed). Complete any loopback redirect async so login can keep
   * waiting.
   */
  openExternalBrowserCallback?: (url: string) => void | Promise<void>;
};
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// Not exported, so only deserializeConnection can create a connection with deferred init.
const DESERIALIZED_PAYLOAD = Symbol('deserializedPayload');
type DeserializedConnectionOptions = ConnectionOptions & { [DESERIALIZED_PAYLOAD]?: string };

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
  fetchAsString?: DataType[];
}

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
export class Connection {
  #core: CoreConnectionInstance;
  #defaultRowOptions: RowOptions;
  #id: string;
  // Original deserialize payload. Core has no tokens until connection_init, so
  // serialize() before the first use reads this instead of an empty tokenInfo.
  #serializedBeforeInit?: string;

  constructor(options: ConnectionOptions) {
    const {
      rowMode,
      fetchAsString,
      jsTreatIntegerAsBigInt,
      representNullAsStringNull,
      arrayBindingThreshold,
      openExternalBrowserCallback,
      [DESERIALIZED_PAYLOAD]: serializedBeforeInit,
      ...coreOptions
    } = options as DeserializedConnectionOptions;
    this.#id = randomUUID();

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
      normalizeConnectionOptions({
        ...coreOptions,
        useEnvProxy: GlobalConfig.useEnvProxy,
      }),
      sessionParameters,
      openExternalBrowserCallback,
      serializedBeforeInit !== undefined,
    );
    this.#serializedBeforeInit = serializedBeforeInit;
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
    const info = this.#core.getTokenInfo();
    if (info.sessionToken) {
      this.#serializedBeforeInit = undefined;
      return serializeTokenInfo(info);
    }
    if (this.#serializedBeforeInit && this.isUp()) {
      return this.#serializedBeforeInit;
    }
    return serializeTokenInfo({});
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
    const bindings = selectBindPayload(
      options.binds,
      this.#core.getSessionParameters().clientStageArrayBindingThreshold,
    );

    let parameters = options.parameters
      ? Object.fromEntries(
          Object.entries(options.parameters).map(([key, value]) => [key, String(value)]),
        )
      : null;
    if (options.cwd) {
      parameters = { ...parameters, cwd: options.cwd };
    }

    return this.#runStatement(
      this.#core.execute({
        query: options.sqlText,
        bindings: bindings ?? undefined,
        parameters: parameters ?? undefined,
        asyncExec: options.asyncExec,
        requestId: options.requestId,
        describeOnly: options.describeOnly,
      }),
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
    return coreIsStillRunning(status);
  }

  isAnError(status: QueryStatus): boolean {
    return coreIsAnError(status);
  }

  fetchResult(options: FetchResultOptions): RowStatement {
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

/** Reports linked Rustls TLS provider status without creating a connection. */
export const getTlsStatus: () => TlsStatus = coreGetTlsStatus;

// TODO:
// - JSDoc needed
// - Map to similar shape as old driver where we have core object that has bunch of methods and
//   it is exported as default
export const configure = (options: ConfigureOptions) => updateGlobalConfig(options);
export const createConnection = (options: ConnectionOptions) => new Connection(options);
export const serializeConnection = (connection: Connection): string => connection.serialize();
/**
 * Rebuilds a connection from a string produced by {@link Connection.serialize}.
 * A payload that contains tokens addresses the same server session as the
 * connection that produced the string. `destroy()` on either connection logs
 * that session out.
 */
export const deserializeConnection = (
  options: ConnectionOptions,
  serializedConnection?: unknown,
): Connection => {
  const tokenInfo = deserializeTokenInfo(serializedConnection);
  if (
    !tokenInfo.sessionToken ||
    !tokenInfo.masterToken ||
    typeof serializedConnection !== 'string'
  ) {
    return new Connection(options);
  }
  const deserializedOptions: DeserializedConnectionOptions = {
    ...options,
    sessionToken: tokenInfo.sessionToken,
    masterToken: tokenInfo.masterToken,
    [DESERIALIZED_PAYLOAD]: serializedConnection,
  };
  return new Connection(deserializedOptions);
};

export default {
  configure,
  getTlsStatus,
  createConnection,
  createPool,
  serializeConnection,
  deserializeConnection,
  ErrorCode,
  ocspModes,
};
