import type { CoreColumnInstance, CoreKnownSessionParameters } from '../core/index.js';
import type { CellConverter, ConverterFactory, DataType, RowOptions } from './types.js';
import { resolveColumnNames } from './column-names.js';
import {
  binaryAsStringConverter,
  booleanAsStringConverter,
  dateAsStringConverter,
  realAsStringConverter,
  textAsStringConverter,
  timestampLtzAsStringConverter,
  timestampNtzAsStringConverter,
  timestampTzAsStringConverter,
  vectorAsStringConverter,
} from './string-converters.js';
import {
  dateConverter,
  createFixedConverter,
  timestampLtzConverter,
  timestampNtzConverter,
  timestampTzConverter,
  variantConverter,
} from './value-converters.js';

const CONVERTER_FACTORIES_BY_COLUMN_TYPE: Record<
  string,
  {
    asValue: ConverterFactory | null;
    asString: ConverterFactory | null;
  }
> = {
  text: { asValue: null, asString: () => textAsStringConverter },
  fixed: { asValue: createFixedConverter, asString: () => textAsStringConverter },
  real: { asValue: null, asString: () => realAsStringConverter },
  vector: { asValue: null, asString: () => vectorAsStringConverter },
  decfloat: { asValue: null, asString: () => textAsStringConverter },
  interval_year_month: { asValue: null, asString: () => textAsStringConverter },
  interval_day_time: { asValue: null, asString: () => textAsStringConverter },
  boolean: { asValue: null, asString: () => booleanAsStringConverter },
  binary: { asValue: null, asString: () => binaryAsStringConverter },
  date: { asValue: dateConverter, asString: dateAsStringConverter },
  timestamp_tz: { asValue: timestampTzConverter, asString: timestampTzAsStringConverter },
  timestamp_ltz: { asValue: timestampLtzConverter, asString: timestampLtzAsStringConverter },
  timestamp_ntz: { asValue: timestampNtzConverter, asString: timestampNtzAsStringConverter },
  variant: { asValue: () => variantConverter, asString: () => textAsStringConverter },
  object: { asValue: () => variantConverter, asString: null },
  array: { asValue: () => variantConverter, asString: null },
  map: { asValue: () => variantConverter, asString: null },
};

const COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN: Record<DataType, string[]> = {
  String: ['text', 'decfloat', 'interval_year_month', 'interval_day_time'],
  Number: ['fixed', 'real', 'vector'],
  Boolean: ['boolean'],
  Buffer: ['binary'],
  Date: ['date', 'timestamp_tz', 'timestamp_ltz', 'timestamp_ntz'],
  JSON: ['variant'],
};

function createColumnConverter(
  column: CoreColumnInstance,
  asStringColumnTypes: ReadonlySet<string>,
  sessionParameters: CoreKnownSessionParameters,
  options: { representNullAsStringNull: boolean },
): CellConverter | null {
  const columnType = column.getType();
  const factories = CONVERTER_FACTORIES_BY_COLUMN_TYPE[columnType];
  if (!factories) {
    return null;
  }

  if (!asStringColumnTypes.has(columnType)) {
    return factories.asValue?.(column, sessionParameters) ?? null;
  }

  const asString = factories.asString?.(column, sessionParameters) ?? null;
  if (!asString) {
    return null;
  }

  return (value) => {
    if (value === null && options.representNullAsStringNull === false) {
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
}

type RowFormatter = (rawRow: unknown[]) => unknown[] | Record<string, unknown>;

export function createRowFormatter({
  columns,
  sessionParameters,
  rowOptions,
}: RowFormatterOptions): RowFormatter {
  const columnNames = resolveColumnNames(columns, rowOptions.rowMode);
  const asStringColumnTypes = new Set(
    rowOptions.fetchAsString.flatMap((token) => COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN[token]),
  );

  const columnConverters: ColumnConverter[] = [];
  for (const column of columns) {
    const convert = createColumnConverter(column, asStringColumnTypes, sessionParameters, {
      representNullAsStringNull: rowOptions.representNullAsStringNull,
    });
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
