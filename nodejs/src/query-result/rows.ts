import { Readable } from 'node:stream';
import type { CoreConnectionInstance, CoreStatementInstance } from '../core/index.js';
import type { RowMode } from './types.js';
import { createRowMapper } from './cell-mapping.js';
import { resolveColumnNames } from './column-names.js';

// TODO:
// Consider combining remapRow and shapeRow into 1 util function
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
    while (await coreStatement.fetchNextBatch()) {
      let row: unknown[] | null = null;
      while ((row = coreStatement.getNextRow()) !== null) {
        remapRow(row);
        rows.push(shapeRow(row, columnNames, rowMode));
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
  rowMode: RowMode,
): Readable {
  const columns = coreStatement.getColumns()!;
  const columnNames = resolveColumnNames(columns, rowMode);
  const remapRow = createRowMapper(columns, connection);

  // Decodes one row out of the resident batch, returning false once it is
  // drained and the stream needs a refill.
  const pushNextRow = (stream: Readable): boolean => {
    const row = coreStatement.getNextRow();
    if (row === null) {
      return false;
    }
    remapRow(row);
    stream.push(shapeRow(row, columnNames, rowMode));
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
