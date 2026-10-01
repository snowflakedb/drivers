import type { CellConverter, ConverterFactory } from './types.js';
import { GlobalConfig } from '../global-config.js';
import { SnowflakeDate } from './SnowflakeDate.js';

interface TimestampCell {
  epochMillis: number;
  nanos: number;
  offsetMinutes?: number;
}

const toNumber = (value: unknown) => (value === null ? null : Number(value));
const toBigInt = (value: unknown) => (value === null ? null : BigInt(value as string));

export const createFixedConverter: ConverterFactory = (column, { jsTreatIntegerAsBigInt }) =>
  jsTreatIntegerAsBigInt && column.getScale() === 0 ? toBigInt : toNumber;

export const timestampTzConverter: ConverterFactory = (column, { timestampTzOutputFormat }) => {
  const scale = column.getScale()!;
  return (value) => {
    if (value === null || value === undefined) {
      return value;
    }
    const cell = value as Required<TimestampCell>;
    return new SnowflakeDate({
      value: cell.epochMillis,
      nanoSeconds: cell.nanos,
      scale,
      timezone: cell.offsetMinutes,
      format: timestampTzOutputFormat,
    });
  };
};

export const timestampLtzConverter: ConverterFactory = (
  column,
  { timestampLtzOutputFormat, timezone },
) => {
  const scale = column.getScale()!;
  return (value) => {
    if (value === null || value === undefined) {
      return value;
    }
    const cell = value as TimestampCell;
    return new SnowflakeDate({
      value: cell.epochMillis,
      nanoSeconds: cell.nanos,
      scale,
      timezone,
      format: timestampLtzOutputFormat,
    });
  };
};

export const timestampNtzConverter: ConverterFactory = (column, { timestampNtzOutputFormat }) => {
  const scale = column.getScale()!;
  return (value) => {
    if (value === null || value === undefined) {
      return value;
    }
    const cell = value as TimestampCell;
    return new SnowflakeDate({
      value: cell.epochMillis,
      nanoSeconds: cell.nanos,
      scale,
      timezone: 'UTC',
      format: timestampNtzOutputFormat,
    });
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
  } catch {
    return GlobalConfig.xmlColumnVariantParser(text);
  }
};
