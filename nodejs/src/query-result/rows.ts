import { Readable } from 'node:stream';
import type { CoreConnectionInstance, CoreStatementInstance } from '../core/index.js';
import type { RowMode } from './types.js';
import { createRowMapper } from './cell-mapping.js';
import { resolveColumnNames } from './column-names.js';

// `row` is shaped in place after `createRowMapper`'s content transforms already ran on it.
function shapeRow(row: unknown[], columnNames: string[], rowMode: RowMode): unknown {
  if (rowMode === 'array') {
    return row;
  }
  return row.reduce<Record<string, unknown>>((shaped, cell, index) => {
    shaped[columnNames[index]] = cell;
    return shaped;
  }, {});
}

export async function collectRows(
  connection: CoreConnectionInstance,
  coreStatement: CoreStatementInstance,
  rowMode: RowMode,
): Promise<unknown[]> {
  try {
    await coreStatement.waitForCompletion();
    const columns = coreStatement.getColumns()!;
    const columnNames = resolveColumnNames(columns, rowMode);
    const remapRow = createRowMapper(columns, connection);

    const rows: unknown[] = [];
    while (true) {
      const row = await coreStatement.getNextRow();
      if (row === null) {
        break;
      }
      remapRow(row);
      rows.push(shapeRow(row, columnNames, rowMode));
    }

    return rows;
  } finally {
    coreStatement.close();
  }
}

export function createRowStream(
  connection: CoreConnectionInstance,
  coreStatement: CoreStatementInstance,
  rowMode: RowMode,
): Readable {
  const columns = coreStatement.getColumns()!;
  const columnNames = resolveColumnNames(columns, rowMode);
  const remapRow = createRowMapper(columns, connection);
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
          remapRow(row);
          this.push(shapeRow(row, columnNames, rowMode));
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
