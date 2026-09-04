import type { CoreColumnInstance, CoreConnectionInstance } from '../core/index.js';
import type { CellConverter, ConversionContext, DataType, RowOptions } from './types.js';
import { resolveColumnNames } from './column-names.js';
import { readSessionParameters } from './session-parameters.js';
import {
  binaryAsStringConverter,
  booleanAsStringConverter,
  dateAsStringConverter,
  realAsStringConverter,
  textAsStringConverter,
} from './string-converters.js';
import { fixedConverter, variantConverter } from './value-converters.js';

const CONVERTERS_BY_COLUMN_TYPE: Record<
  string,
  {
    asValue: CellConverter | null;
    asString: CellConverter | null;
  }
> = {
  text: { asValue: null, asString: textAsStringConverter },
  fixed: { asValue: fixedConverter, asString: textAsStringConverter },
  real: { asValue: null, asString: realAsStringConverter },
  decfloat: { asValue: null, asString: textAsStringConverter },
  boolean: { asValue: null, asString: booleanAsStringConverter },
  binary: { asValue: null, asString: binaryAsStringConverter },
  date: { asValue: null, asString: dateAsStringConverter },
  variant: { asValue: variantConverter, asString: textAsStringConverter },
  object: { asValue: variantConverter, asString: null },
  array: { asValue: variantConverter, asString: null },
  map: { asValue: variantConverter, asString: null },
};

const COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN: Record<DataType, string[]> = {
  String: ['text', 'decfloat'],
  Number: ['fixed', 'real'],
  Boolean: ['boolean'],
  Buffer: ['binary'],
  Date: ['date'],
  JSON: ['variant'],
};

function selectConverter(
  column: CoreColumnInstance,
  asStringColumnTypes: ReadonlySet<string>,
  options: { representNullAsStringNull: boolean },
): CellConverter | null {
  const columnType = column.getType();
  const converters = CONVERTERS_BY_COLUMN_TYPE[columnType];
  if (!converters) {
    return null;
  }
  if (!asStringColumnTypes.has(columnType)) {
    return converters.asValue;
  }
  // Only the asString converters render a NULL as the string 'NULL'; when
  // representNullAsStringNull is off, short-circuit the NULL back to real null
  // here so a new asString converter cannot forget to honor the option.
  const asString = converters.asString;
  if (asString === null || options.representNullAsStringNull) {
    return asString;
  }
  return (value, context) => (value === null ? null : asString(value, context));
}

interface ColumnConverter {
  index: number;
  convert: CellConverter;
  context: ConversionContext;
}

interface RowFormatterOptions {
  columns: CoreColumnInstance[];
  connection: CoreConnectionInstance;
  rowOptions: RowOptions;
}

type RowFormatter = (rawRow: unknown[]) => unknown[] | Record<string, unknown>;

export function createRowFormatter({
  columns,
  connection,
  rowOptions,
}: RowFormatterOptions): RowFormatter {
  const columnNames = resolveColumnNames(columns, rowOptions.rowMode);
  const asStringColumnTypes = new Set(
    rowOptions.fetchAsString.flatMap((token) => COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN[token]),
  );
  const { treatIntegerAsBigInt } = readSessionParameters(connection);

  const columnConverters: ColumnConverter[] = [];
  for (const column of columns) {
    const convert = selectConverter(column, asStringColumnTypes, {
      representNullAsStringNull: rowOptions.representNullAsStringNull,
    });
    if (convert !== null) {
      columnConverters.push({
        index: column.getIndex(),
        convert,
        context: { scale: column.getScale(), treatIntegerAsBigInt },
      });
    }
  }

  return (row) => {
    // The bridge builds a fresh row array per call, so converting cells in
    // place is safe: nothing else holds a reference to this array.
    for (const { index, convert, context } of columnConverters) {
      row[index] = convert(row[index], context);
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
