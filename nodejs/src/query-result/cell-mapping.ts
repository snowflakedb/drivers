import type { CoreColumnInstance, CoreKnownSessionParameters } from '../core/index.js';
import type {
  CellConverter,
  ConverterContext,
  ConverterFactory,
  DataType,
  RowOptions,
} from './types.js';
import { createFixedConverter, variantConverter } from './value-converters.js';

export const CONVERTER_FACTORIES_BY_COLUMN_TYPE: Record<string, ConverterFactory> = {
  fixed: createFixedConverter,
  variant: () => variantConverter,
  object: () => variantConverter,
  array: () => variantConverter,
  map: () => variantConverter,
};

// When one of these tokens is in `fetchAsString`, the bridge already returns the listed column
// types as strings, so their value converter must not run.
export const COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN: Record<DataType, string[]> = {
  String: [],
  Number: ['fixed'],
  Boolean: [],
  Buffer: [],
  Date: [],
  JSON: ['variant', 'object', 'array', 'map'],
};

function createColumnConverter(
  column: CoreColumnInstance,
  asStringColumnTypes: ReadonlySet<string>,
  context: ConverterContext,
): CellConverter | null {
  const columnType = column.getType();
  const factory = CONVERTER_FACTORIES_BY_COLUMN_TYPE[columnType];
  if (!factory || asStringColumnTypes.has(columnType)) {
    return null;
  }
  return factory(column, context);
}

interface ColumnConverter {
  index: number;
  convert: CellConverter;
}

interface RowFormatterOptions {
  columns: CoreColumnInstance[];
  sessionParameters: CoreKnownSessionParameters;
  rowOptions: RowOptions;
  onPrecisionLoss: ConverterContext['onPrecisionLoss'];
}

type RowFormatter = (rawRow: unknown[]) => unknown[] | Record<string, unknown>;

export function createRowFormatter({
  columns,
  sessionParameters,
  rowOptions,
  onPrecisionLoss,
}: RowFormatterOptions): RowFormatter {
  const context: ConverterContext = {
    sessionParameters,
    options: {
      representNullAsStringNull: rowOptions.representNullAsStringNull,
    },
    onPrecisionLoss,
  };
  const columnNames = columns.map((column) => column.getName());
  const asStringColumnTypes = new Set(
    rowOptions.fetchAsString.flatMap((token) => COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN[token]),
  );

  const columnConverters: ColumnConverter[] = [];
  for (const column of columns) {
    const convert = createColumnConverter(column, asStringColumnTypes, context);
    if (convert !== null) {
      columnConverters.push({ index: column.getIndex(), convert });
    }
  }

  return (row) => {
    // The bridge builds a fresh row array per call, so converting cells in
    // place is safe: nothing else holds a reference to this array.
    for (const { index, convert } of columnConverters) {
      row[index] = convert(row[index]);
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
