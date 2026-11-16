import type { CoreColumnInstance } from '../core/index.js';
import type { RowOptions } from './types.js';
import { variantConverter } from './value-converters.js';

const JSON_COLUMN_TYPES = new Set(['variant', 'object', 'array', 'map']);

interface RowFormatterOptions {
  columns: CoreColumnInstance[];
  rowOptions: RowOptions;
}

type RowFormatter = (rawRow: unknown[]) => unknown[] | Record<string, unknown>;

export function createRowFormatter({ columns, rowOptions }: RowFormatterOptions): RowFormatter {
  const columnNames = columns.map((column) => column.getName());

  // The bridge returns every cell as its final JS value, except VARIANT, OBJECT, ARRAY and MAP.
  // Those arrive as JSON text and are parsed here, because the parser is the JS function the
  // user can set with `snowflake.configure({ jsonColumnVariantParser, xmlColumnVariantParser })`.
  // With `fetchAsString: ['JSON']` the text is returned as is.
  const jsonColumnIndexes: number[] = [];
  if (!rowOptions.fetchAsString.includes('JSON')) {
    for (const column of columns) {
      if (JSON_COLUMN_TYPES.has(column.getType())) {
        jsonColumnIndexes.push(column.getIndex());
      }
    }
  }

  return (row) => {
    // The bridge builds a fresh row array per call, so converting cells in
    // place is safe: nothing else holds a reference to this array.
    for (const index of jsonColumnIndexes) {
      row[index] = variantConverter(row[index]);
    }
    if (rowOptions.rowMode === 'array') {
      return row;
    }
    const shaped: Record<string, unknown> = {};
    for (let index = 0; index < row.length; index++) {
      shaped[columnNames[index]] = row[index];
    }
    return shaped;
  };
}
