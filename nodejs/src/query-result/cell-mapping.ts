import type { CoreColumnInstance, CoreKnownSessionParameters } from '../core/index.js';
import type {
  CellConverter,
  ConverterContext,
  ConverterFactory,
  DataType,
  RowOptions,
} from './types.js';
import { realAsStringConverter, vectorAsStringConverter } from './string-converters.js';
import { createFixedConverter, variantConverter } from './value-converters.js';

export const CONVERTER_FACTORIES_BY_COLUMN_TYPE: Record<
  string,
  {
    asValue: ConverterFactory | null;
    asString: ConverterFactory | null;
  }
> = {
  fixed: { asValue: createFixedConverter, asString: null },
  real: { asValue: null, asString: () => realAsStringConverter },
  vector: { asValue: null, asString: () => vectorAsStringConverter },
  variant: { asValue: () => variantConverter, asString: null },
  object: { asValue: () => variantConverter, asString: null },
  array: { asValue: () => variantConverter, asString: null },
  map: { asValue: () => variantConverter, asString: null },
};

export const COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN: Record<DataType, string[]> = {
  // Column types listed here skip their `asValue` converter when the token is requested.
  // REAL and VECTOR are converted to strings by their `asString` converter. FIXED and the JSON
  // types are already strings from the bridge and are listed only so `Number()`/JSON parsing does not
  // run on them. Every other type is converted to a string in the bridge.
  String: [],
  Number: ['fixed', 'real', 'vector'],
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
  const factories = CONVERTER_FACTORIES_BY_COLUMN_TYPE[columnType];
  if (!factories) {
    return null;
  }

  if (!asStringColumnTypes.has(columnType)) {
    return factories.asValue?.(column, context) ?? null;
  }

  const asString = factories.asString?.(column, context) ?? null;
  if (!asString) {
    return null;
  }

  return (value) => {
    if (value === null && !context.options.representNullAsStringNull) {
      return null;
    }
    return asString(value);
  };
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
