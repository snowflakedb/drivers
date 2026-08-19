import { Readable } from 'node:stream';
import type { CoreStatementInstance } from '../core/index.js';
import { resolveColumnNames, reshapeRowForMode, type RowMode } from './row-mode.js';

function getColumnNames(coreStatement: CoreStatementInstance): string[] {
  return (coreStatement.getColumns() ?? []).map((column) => column.getName());
}

export async function collectRows(
  coreStatement: CoreStatementInstance,
  rowMode: RowMode,
): Promise<unknown[]> {
  try {
    const rows: unknown[] = [];
    let columnNames: string[] | undefined;
    while (true) {
      const row = await coreStatement.getNextRow();
      if (row === null) {
        break;
      }
      columnNames ??= resolveColumnNames(getColumnNames(coreStatement), rowMode);
      rows.push(reshapeRowForMode(row, columnNames, rowMode));
    }
    return rows;
  } finally {
    coreStatement.close();
  }
}

export function createRowStream(coreStatement: CoreStatementInstance, rowMode: RowMode): Readable {
  let columnNames: string[] | undefined;
  return new Readable({
    objectMode: true,
    read() {
      coreStatement
        .getNextRow()
        .then((row) => {
          if (row === null) {
            this.push(null);
            return;
          }
          columnNames ??= resolveColumnNames(getColumnNames(coreStatement), rowMode);
          this.push(reshapeRowForMode(row, columnNames, rowMode));
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
