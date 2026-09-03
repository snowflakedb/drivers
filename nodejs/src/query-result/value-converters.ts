import type { CellConverter } from './types.js';
import { GlobalConfig } from '../global-config.js';

const toNumber = (value: unknown) => (value === null ? null : Number(value));
const toBigInt = (value: unknown) => (value === null ? null : BigInt(value as string));

export const fixedConverter: CellConverter = (value, { scale, treatIntegerAsBigInt }) =>
  treatIntegerAsBigInt && scale === 0 ? toBigInt(value) : toNumber(value);

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
