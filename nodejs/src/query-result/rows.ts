import { Readable } from 'node:stream';
import type { CoreConnectionInstance, CoreStatementInstance } from '../core/index.js';
import { logger } from '../logging/index.js';
import { createRowFormatter } from './cell-mapping.js';
import { shouldRenameDuplicateColumns, type RowOptions } from './types.js';

const precisionLossWarning = (queryId: string | null, columnIndex: number) =>
  `
Query ${queryId}: Query result precision loss detected when converting a value in column at index ${columnIndex} to a JavaScript Number.

Fetch as string or BigInt instead:
https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume#fetching-integer-data-types-as-bigint
https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume#fetching-data-types-as-strings

This will throw an error in future driver versions.
`.trim();

function createStatementRowFormatter(
  connection: CoreConnectionInstance,
  coreStatement: CoreStatementInstance,
  rowOptions: RowOptions,
) {
  const queryId = coreStatement.getQueryId();
  // each handler fires per affected value, but its telemetry event is sent once per query
  const reportedEventTypes = new Set<string>();
  const sendTelemetryEventOnce = (eventType: string) => {
    if (reportedEventTypes.has(eventType)) {
      return;
    }
    reportedEventTypes.add(eventType);
    // telemetry is best-effort: a failed send does not reject into the user's process
    connection
      .sendTelemetryEvent(eventType, queryId)
      .catch((err: Error) => logger.debug(`Failed to send ${eventType} telemetry: ${err}`));
  };

  return createRowFormatter({
    columns: coreStatement.getColumns(shouldRenameDuplicateColumns(rowOptions.rowMode))!,
    sessionParameters: coreStatement.getSessionParametersSnapshot(),
    rowOptions,
    onPrecisionLoss: (columnIndex) => {
      logger.warn(precisionLossWarning(queryId, columnIndex));
      sendTelemetryEventOnce('selecting_with_precision_loss');
    },
  });
}

export async function collectRows(
  connection: CoreConnectionInstance,
  coreStatement: CoreStatementInstance,
  rowOptions: RowOptions,
): Promise<unknown[]> {
  try {
    const formatRow = createStatementRowFormatter(connection, coreStatement, rowOptions);

    const rows: unknown[] = [];
    while (await coreStatement.fetchNextBatch(rowOptions)) {
      let row: unknown[] | null = null;
      while ((row = coreStatement.getNextRow()) !== null) {
        rows.push(formatRow(row));
      }
    }

    return rows;
  } finally {
    coreStatement.close();
  }
}

export function createRowStream(
  connection: CoreConnectionInstance,
  coreStatement: CoreStatementInstance,
  rowOptions: RowOptions,
): Readable {
  const formatRow = createStatementRowFormatter(connection, coreStatement, rowOptions);

  const pushNextRow = (stream: Readable): boolean => {
    const row = coreStatement.getNextRow();
    if (row === null) {
      return false;
    }
    stream.push(formatRow(row));
    return true;
  };

  return new Readable({
    objectMode: true,
    highWaterMark: rowOptions.rowStreamHighWaterMark,
    read() {
      if (pushNextRow(this)) {
        return;
      }

      coreStatement
        .fetchNextBatch(rowOptions)
        .then((hasRows) => {
          if (hasRows) {
            pushNextRow(this);
          } else {
            this.push(null);
          }
        })
        .catch((err: Error) => {
          this.destroy(err);
        });
    },
    destroy(err, callback) {
      if (!coreStatement.hasNext()) {
        coreStatement.close();
      }
      callback(err);
    },
  });
}
