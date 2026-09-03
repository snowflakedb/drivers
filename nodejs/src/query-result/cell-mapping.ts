import type { CoreConnectionInstance, CoreColumnInstance } from '../core/index.js';
import type { CellConverter, ConversionContext, DataType, RowMode } from './types.js';
import SessionParameterName from '../constants/SessionParameterName.js';
import { resolveColumnNames } from './column-names.js';
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

function isSessionParameterEnabled(connection: CoreConnectionInstance, name: string): boolean {
  return connection.getSessionParameter(name)?.toLowerCase() === 'true';
}

function selectConverter(
  column: CoreColumnInstance,
  asStringColumnTypes: ReadonlySet<string>,
): CellConverter | null {
  const columnType = column.getType();
  const converters = CONVERTERS_BY_COLUMN_TYPE[columnType];
  if (!converters) {
    return null;
  }
  return asStringColumnTypes.has(columnType) ? converters.asString : converters.asValue;
}

interface ColumnConverter {
  index: number;
  convert: CellConverter;
  context: ConversionContext;
}

interface RowFormatterOptions {
  columns: CoreColumnInstance[];
  connection: CoreConnectionInstance;
  rowMode: RowMode;
  fetchAsString?: DataType[];
}

type RowFormatter = (rawRow: unknown[]) => unknown[] | Record<string, unknown>;

export function createRowFormatter({
  columns,
  connection,
  rowMode,
  fetchAsString,
}: RowFormatterOptions): RowFormatter {
  const columnNames = resolveColumnNames(columns, rowMode);
  const asStringColumnTypes = new Set(
    (fetchAsString ?? []).flatMap((token) => COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN[token]),
  );
  const treatIntegerAsBigInt = isSessionParameterEnabled(
    connection,
    SessionParameterName.JS_TREAT_INTEGER_AS_BIGINT,
  );

  const columnConverters: ColumnConverter[] = [];
  for (const column of columns) {
    const convert = selectConverter(column, asStringColumnTypes);
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
    if (rowMode === 'array') {
      return row;
    }
    const shaped: Record<string, unknown> = {};
    for (let index = 0; index < row.length; index++) {
      shaped[columnNames[index]] = row[index];
    }
    return shaped;
  };
}
