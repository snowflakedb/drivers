import { Readable } from 'node:stream';
import type { CoreColumnInstance, CoreStatementInstance } from '../core/index.js';
import type { RowMode } from './types.js';
import { GlobalConfig } from '../global-config.js';
import { resolveColumnNames } from './column-names.js';

function transformCell(cell: unknown, column: CoreColumnInstance): unknown {
  if (column.isVariant()) {
    if (cell === null || cell === undefined) {
      return cell;
    }
    if (cell === '') {
      return undefined;
    }
    const value = cell as string;
    try {
      return GlobalConfig.jsonColumnVariantParser(value);
    } catch {
      return GlobalConfig.xmlColumnVariantParser(value);
    }
  }
  return cell;
}

function transformRow({
  row,
  columns,
  columnNames,
  rowMode,
}: {
  row: unknown[];
  columns: CoreColumnInstance[];
  columnNames: string[];
  rowMode: RowMode;
}): unknown {
  if (rowMode === 'array') {
    return row.map((cell, index) => transformCell(cell, columns[index]));
  }
  return row.reduce<Record<string, unknown>>((shaped, cell, index) => {
    shaped[columnNames[index]] = transformCell(cell, columns[index]);
    return shaped;
  }, {});
}

export async function collectRows(
  coreStatement: CoreStatementInstance,
  rowMode: RowMode,
): Promise<unknown[]> {
  try {
    await coreStatement.waitForCompletion();
    const columns = coreStatement.getColumns()!;
    const columnNames = resolveColumnNames(columns, rowMode);

    const rows: unknown[] = [];
    while (true) {
      const row = await coreStatement.getNextRow();
      if (row === null) {
        break;
      }
      rows.push(transformRow({ row, columns, columnNames, rowMode }));
    }

    return rows;
  } finally {
    coreStatement.close();
  }
}

export function createRowStream(coreStatement: CoreStatementInstance, rowMode: RowMode): Readable {
  const columns = coreStatement.getColumns()!;
  const columnNames = resolveColumnNames(columns, rowMode);
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
          this.push(transformRow({ row, columns, columnNames, rowMode }));
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
