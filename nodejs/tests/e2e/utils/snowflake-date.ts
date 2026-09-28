import { SnowflakeDate } from 'snowflake-sdk';
import { expect } from 'vitest';

const DEFAULT_TIMESTAMP_SCALE = 9;

function dateTimeToMillis(when: string | number): number {
  return typeof when === 'number' ? when : new Date(`${when}Z`).getTime();
}

export function expectSnowflakeDate(values: unknown[], expected: (SnowflakeDate | null)[]): void {
  expect(values).toHaveLength(expected.length);
  expected.forEach((entry, index) => {
    const value = values[index];
    if (entry === null) {
      expect(value).toBeNull();
      return;
    }
    const actual = value as SnowflakeDate;
    expect(actual).toBeInstanceOf(Date);
    expect(actual).toEqual(entry);
    expect(actual.getNanoSeconds()).toBe(entry.getNanoSeconds());
    expect(actual.getScale()).toBe(entry.getScale());
    expect(actual.getTimezone()).toBe(entry.getTimezone());
  });
}

export function createTestDate(value: string | number): SnowflakeDate {
  return new SnowflakeDate({
    value: typeof value === 'number' ? value : new Date(`${value}T00:00:00.000Z`).getTime(),
    nanoSeconds: 0,
    scale: 0,
    timezone: 'UTC',
    format: 'YYYY-MM-DD',
  });
}

export function createTestLtzDate(
  value: string | number,
  options: {
    nanoSeconds?: number;
    scale?: number;
    timezone?: string;
  } = {},
): SnowflakeDate {
  return new SnowflakeDate({
    value: dateTimeToMillis(value),
    nanoSeconds: options.nanoSeconds ?? 0,
    scale: options.scale ?? DEFAULT_TIMESTAMP_SCALE,
    timezone: options.timezone ?? 'UTC',
    format: '',
  });
}

export function createTestTzDate(
  value: string | number,
  options: {
    offsetMinutes?: number;
    nanoSeconds?: number;
    scale?: number;
  } = {},
): SnowflakeDate {
  return new SnowflakeDate({
    value: dateTimeToMillis(value),
    nanoSeconds: options.nanoSeconds ?? 0,
    scale: options.scale ?? DEFAULT_TIMESTAMP_SCALE,
    timezone: options.offsetMinutes ?? 0,
    format: '',
  });
}

export function createTestNtzDate(
  value: string | number,
  options: {
    nanoSeconds?: number;
    scale?: number;
  } = {},
): SnowflakeDate {
  return createTestLtzDate(value, { ...options, timezone: 'UTC' });
}
