import type { Binds } from '../query-result/binds.js';
import type { DataType, RowMode, StatementCallback } from '../query-result/types.js';

// This should be called StatementOptions or ExecuteStatementOptions but we keep the name
// for backwards compatibility with old driver.
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
