import { describe, it, expect } from 'vitest';
import { buildBindsMap, countBoundValues, type Binds } from '../../../src/query-result/binds.js';

describe('buildBindsMap', () => {
  it('should infer the logical type per column of a single row', () => {
    expect(buildBindsMap([true, 42, 3.14, 'x', null])).toEqual({
      '1': { type: 'BOOLEAN', value: 'true' },
      '2': { type: 'FIXED', value: '42' },
      '3': { type: 'REAL', value: '3.14' },
      '4': { type: 'TEXT', value: 'x' },
      '5': { type: 'TEXT', value: null },
    });
  });

  it('should serialize objects and arrays as VARIANT JSON', () => {
    const binds = [{ a: 1 }, [1, 2]] as unknown as Binds;
    expect(buildBindsMap(binds)).toEqual({
      '1': { type: 'VARIANT', value: '{"a":1}' },
      '2': { type: 'VARIANT', value: '[1,2]' },
    });
  });

  it('should serialize a Date via toJSON as VARIANT', () => {
    const binds = [new Date('2021-01-01T00:00:00.000Z')] as unknown as Binds;
    expect(buildBindsMap(binds)).toEqual({
      '1': { type: 'VARIANT', value: '2021-01-01T00:00:00.000Z' },
    });
  });

  it('should transpose multi-row binds into a per-column value array', () => {
    expect(
      buildBindsMap([
        [1, 'a'],
        [2, 'b'],
        [3, 'c'],
      ]),
    ).toEqual({
      '1': { type: 'FIXED', value: ['1', '2', '3'] },
      '2': { type: 'TEXT', value: ['a', 'b', 'c'] },
    });
  });

  it('should infer the column type from the first row of multi-row binds', () => {
    expect(buildBindsMap([[1.5], [2]])).toEqual({
      '1': { type: 'REAL', value: ['1.5', '2'] },
    });
  });

  it('should keep null as null in every position rather than the string "null"', () => {
    expect(buildBindsMap([null])).toEqual({
      '1': { type: 'TEXT', value: null },
    });
    expect(
      buildBindsMap([
        [1, null],
        [2, 'x'],
      ]),
    ).toEqual({
      '1': { type: 'FIXED', value: ['1', '2'] },
      '2': { type: 'TEXT', value: [null, 'x'] },
    });
  });
});

describe('countBoundValues', () => {
  it('should count a single-row (scalar) bind as zero', () => {
    expect(countBoundValues(['a', 'b', 'c'])).toBe(0);
  });

  it('should count undefined and empty binds as zero', () => {
    expect(countBoundValues(undefined)).toBe(0);
    expect(countBoundValues([])).toBe(0);
  });

  it('should count multi-row binds as rows times columns', () => {
    expect(
      countBoundValues([
        [1, 2],
        [3, 4],
        [5, 6],
      ]),
    ).toBe(6);
  });
});
