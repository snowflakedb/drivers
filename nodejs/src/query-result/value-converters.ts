import type { CellConverter, ConverterFactory } from './types.js';
import { GlobalConfig } from '../global-config.js';

const toNumber = (value: unknown) => (value === null ? null : Number(value));
const toBigInt = (value: unknown) => (value === null ? null : BigInt(value as string));

export const createFixedConverter: ConverterFactory = (column, { jsTreatIntegerAsBigInt }) =>
  jsTreatIntegerAsBigInt && column.getScale() === 0 ? toBigInt : toNumber;

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
