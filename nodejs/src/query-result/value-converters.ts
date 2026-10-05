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
  } catch {
    return GlobalConfig.xmlColumnVariantParser(text);
  }
};
