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
