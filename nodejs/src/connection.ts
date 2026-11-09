import { randomUUID } from 'node:crypto';
import type {
  Connection,
  ConnectionOptions,
  FetchResultOptions,
  RowStatement,
  StatementOption,
} from './public-types/index.js';
import type { StatementCallback, RowOptions, QueryStatus } from './query-result/types.js';
import { toCoreConnectionOptions } from './connection-option-aliases.js';
import { serializeTokenInfo, deserializeTokenInfo } from './connection-serialization.js';
import {
  CoreConnection,
  CoreQueryBindingFormat,
  type CoreConnectionInstance,
  type CoreStatementInstance,
  type ConnectionTokenInfo,
} from './core/index.js';
import { tryWithErrorReporter, type SnowflakeError } from './error.js';
import { GlobalConfig } from './global-config.js';
import { buildBindsMap, selectBindPayload } from './query-result/binds.js';
import { collectRows } from './query-result/rows.js';
import { RowStatementImpl } from './query-result/RowStatement.js';
import { getWrapperIdentity } from './wrapper-identity.js';

export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// Not exported, so session tokens reach the bridge only through deserializeConnection.
const TOKEN_INFO = Symbol('tokenInfo');
interface InternalConnectionOptions extends ConnectionOptions {
  [TOKEN_INFO]?: ConnectionTokenInfo;
}

// TODO: think whether we should have connection class only in bridge that exposes same api as old driver
export class ConnectionImpl implements Connection {
  #core: CoreConnectionInstance;
  #defaultRowOptions: RowOptions;
  #defaultStreamResult: boolean;
  #id: string;

  constructor(options?: ConnectionOptions) {
    const callerOptions = options ?? {};
    const {
      rowMode,
      fetchAsString,
      jsTreatIntegerAsBigInt,
      representNullAsStringNull,
      rowStreamHighWaterMark,
      streamResult,
      arrayBindingThreshold,
      openExternalBrowserCallback,
      browserResponseRenderer,
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
    this.#defaultStreamResult = streamResult ?? false;

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
      browserResponseRenderer,
      tokenInfo,
      noConnectionDetails,
      wrapperIdentity: getWrapperIdentity(),
    });
  }

  connect(callback?: ConnectionCallback): Connection {
    this.connectAsync()
      .then(() => {
        callback?.(undefined, this);
      })
      .catch((err) => {
        callback?.(err, this);
      });
    return this;
  }

  async connectAsync(): Promise<Connection> {
    await this.#core.connect();
    return this;
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

  isValidAsync(): Promise<boolean> {
    return this.#core.isValidAsync();
  }

  execute(options: StatementOption): RowStatement {
    const { binds } = options;
    const threshold = this.#core.getSessionParameters().clientStageArrayBindingThreshold;
    const bindings = tryWithErrorReporter(this.#core, 'api_misuse', () =>
      selectBindPayload(binds, threshold),
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
    const { complete, sqlText, asyncExec, rowOptions } = options;
    const streamResult = options.streamResult ?? this.#defaultStreamResult;
    const statement = new RowStatementImpl(
      this.#core,
      coreStatement,
      rowOptions,
      sqlText,
      complete,
    );
    (async () => {
      try {
        await coreStatement.waitForCompletion();
        if (asyncExec === true || streamResult === true || coreStatement.hasNext()) {
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

  destroy(callback?: ConnectionCallback): void {
    this.#core
      .destroy()
      .then(() => callback?.(undefined, this))
      .catch((err) => callback?.(err, this));
  }
}

export const createConnection = (options?: ConnectionOptions): Connection =>
  new ConnectionImpl(options);
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
  return new ConnectionImpl(internalOptions);
};
