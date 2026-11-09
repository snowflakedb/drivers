import type { ConnectionCallback } from '../connection.js';
import type { QueryStatus } from '../query-result/types.js';
import type { FetchResultOptions } from './FetchResultOptions.js';
import type { RowStatement } from './RowStatement.js';
import type { StatementOption } from './StatementOption.js';

export interface Connection {
  /**
   * Logs in and opens the session. `callback` is called when the login succeeds or fails.
   * Returns this connection.
   */
  connect(callback?: ConnectionCallback): Connection;

  /**
   * Logs in and opens the session. Resolves with this connection when the login succeeds,
   * and rejects when it fails.
   */
  connectAsync(): Promise<Connection>;

  /**
   * Returns a string with the session tokens of this connection.
   * {@link deserializeConnection} turns it into a new connection that uses the same session.
   * The format of the string may change between driver versions, so pass it only to
   * `deserializeConnection` of the same driver version.
   */
  serialize(): string;

  /**
   * Returns true if the connection is active otherwise false.
   */
  isUp(): boolean;

  /**
   * Returns the connection id.
   */
  getId(): string;

  /**
   * Sends a heartbeat to the server, which also renews the session. Resolves `false` when the
   * session is gone rather than rejecting.
   */
  isValidAsync(): Promise<boolean>;

  /**
   * Executes a statement.
   */
  execute(options: StatementOption): RowStatement;

  /**
   * Gets the status of the query based on queryId.
   */
  getQueryStatus(queryId: string): Promise<QueryStatus>;

  /**
   * Gets the status of the query based on queryId and throws if there's an error.
   */
  getQueryStatusThrowIfError(queryId: string): Promise<QueryStatus>;

  /**
   * Checks whether the given status is currently running.
   */
  isStillRunning(status: QueryStatus): boolean;

  /**
   * Checks whether the given status means that there has been an error.
   */
  isAnError(status: QueryStatus): boolean;

  /**
   * Fetches the result of a previously issued statement.
   */
  fetchResult(options: FetchResultOptions): RowStatement;

  /**
   * Waits until a previously issued query finishes, then fetches its result.
   */
  getResultsFromQueryId(options: FetchResultOptions): Promise<RowStatement>;

  /**
   * Closes the connection and logs out of the session.
   */
  destroy(callback?: ConnectionCallback): void;
}
