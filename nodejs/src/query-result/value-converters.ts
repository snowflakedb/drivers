import type { CellConverter, ConverterFactory } from './types.js';
import { GlobalConfig } from '../global-config.js';

const toBigInt = (value: unknown) => (value === null ? null : BigInt(value as string));

export const createFixedConverter: ConverterFactory = (
  column,
  { sessionParameters, onPrecisionLoss },
) => {
  if (sessionParameters.jsTreatIntegerAsBigInt && column.getScale() === 0) {
    return toBigInt;
  }
  return (value) => {
    if (value === null) {
      return null;
    }
    const converted = Number(value);
    // runs the check for every cell (that might be expensive) as in the old driver: any row can hold an unsafe integer
    if (Number.isInteger(converted) && !Number.isSafeInteger(converted)) {
      onPrecisionLoss(column.getIndex());
    }
    return converted;
  };
};

export const variantConverter: CellConverter = (value) => {
  if (value === null || value === undefined) {
    return value;
  }
  if (value === '') {
    return undefined;
  }
  const text = value as string;
  try {
    return GlobalConfig.jsonColumnVariantParser(text);
  } catch (jsonError) {
    try {
      return GlobalConfig.xmlColumnVariantParser(text);
    } catch (xmlError) {
      throw new Error(
        'Variant value cannot be parsed as JSON or XML.\n' +
          ` - JSON parse error: ${errorMessage(jsonError)}\n` +
          ` - XML parse error: ${errorMessage(xmlError)}\n` +
          'If the value contains NaN, Infinity or undefined, set the STRICT_JSON_OUTPUT ' +
          'session parameter to TRUE, or pass a custom jsonColumnVariantParser to snowflake.configure().',
      );
    }
  }
};

const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));
