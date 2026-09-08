import { Readable } from 'node:stream';
import type { CoreStatementInstance } from '../core/index.js';
import type { RowOptions } from './types.js';
import { createRowFormatter } from './cell-mapping.js';

export async function collectRows(
  coreStatement: CoreStatementInstance,
  rowOptions: RowOptions,
): Promise<unknown[]> {
  try {
    await coreStatement.waitForCompletion();
    const columns = coreStatement.getColumns()!;
    const sessionParameters = coreStatement.getSessionParametersSnapshot();
    const formatRow = createRowFormatter({ columns, sessionParameters, rowOptions });

    const rows: unknown[] = [];
    while (await coreStatement.fetchNextBatch()) {
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
  coreStatement: CoreStatementInstance,
  rowOptions: RowOptions,
): Readable {
  const columns = coreStatement.getColumns()!;
  const sessionParameters = coreStatement.getSessionParametersSnapshot();
  const formatRow = createRowFormatter({ columns, sessionParameters, rowOptions });

  // Decodes one row out of the resident batch, returning false once it is
  // drained and the stream needs a refill.
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
    read() {
      if (pushNextRow(this)) {
        return;
      }

      coreStatement
        .fetchNextBatch()
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
      coreStatement.close();
      callback(err);
    },
  });
}
