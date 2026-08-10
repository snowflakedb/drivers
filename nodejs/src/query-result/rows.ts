import { Readable } from 'node:stream';
import type { CoreStatementInstance } from '../core/index.js';

export async function collectRows(coreStatement: CoreStatementInstance): Promise<unknown[]> {
  try {
    const rows: unknown[] = [];
    while (true) {
      const row = await coreStatement.getNextRow();
      if (row === null) {
        break;
      }
      rows.push(row);
    }
    return rows;
  } finally {
    coreStatement.close();
  }
}

export function createRowStream(coreStatement: CoreStatementInstance): Readable {
  return new Readable({
    objectMode: true,
    read() {
      coreStatement
        .getNextRow()
        .then((row) => {
          this.push(row);
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
