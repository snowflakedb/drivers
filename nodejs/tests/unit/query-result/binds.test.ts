import { describe, it, expect } from 'vitest';
import { CoreQueryBindingFormat } from '../../../src/core/index.js';
import {
  buildBindsMap,
  buildBindsCsv,
  countBoundValues,
  selectBindPayload,
  type Binds,
  type InsertBinds,
} from '../../../src/query-result/binds.js';

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

describe('buildBindsCsv', () => {
  it('should serialize rows row-major with a trailing newline', () => {
    expect(
      buildBindsCsv([
        [1, 'a'],
        [2, 'b'],
      ]),
    ).toBe('1,a\n2,b\n');
  });

  it('should render null as a bare empty field and empty string as quoted', () => {
    const binds = [[null, '']] as unknown as InsertBinds;
    expect(buildBindsCsv(binds)).toBe(',""\n');
  });

  it('should render an empty array as an empty string, not "[]"', () => {
    const binds = [[[]]] as unknown as InsertBinds;
    expect(buildBindsCsv(binds)).toBe('""\n');
  });

  it('should quote fields containing comma, quote, backslash, newline, or tab', () => {
    const binds = [['a,b', 'c"d', 'e\\f', 'g\nh', 'i\tj']] as unknown as InsertBinds;
    expect(buildBindsCsv(binds)).toBe('"a,b","c""d","e\\f","g\nh","i\tj"\n');
  });

  it('should serialize a Date via toJSON and a non-empty object via JSON.stringify', () => {
    const binds = [[new Date('2021-01-01T00:00:00.000Z'), { a: 1 }]] as unknown as InsertBinds;
    expect(buildBindsCsv(binds)).toBe('2021-01-01T00:00:00.000Z,"{""a"":1}"\n');
  });
});

describe('selectBindPayload', () => {
  it('should choose the CSV path when the bound-value count exceeds the threshold', () => {
    expect(
      selectBindPayload(
        [
          [1, 2],
          [3, 4],
        ],
        3,
      ),
    ).toEqual({ format: CoreQueryBindingFormat.Csv, data: '1,2\n3,4\n' });
  });

  it('should choose the JSON path when the count equals the threshold (strictly greater-than)', () => {
    expect(
      selectBindPayload(
        [
          [1, 2],
          [3, 4],
        ],
        4,
      ),
    ).toEqual({
      format: CoreQueryBindingFormat.Json,
      data: JSON.stringify(
        buildBindsMap([
          [1, 2],
          [3, 4],
        ]),
      ),
    });
  });

  it('should keep scalar binds inline regardless of the threshold', () => {
    expect(selectBindPayload(['a', 'b'], 0)).toEqual({
      format: CoreQueryBindingFormat.Json,
      data: JSON.stringify(buildBindsMap(['a', 'b'])),
    });
  });

  it('should return null for undefined and empty binds', () => {
    expect(selectBindPayload(undefined, 3)).toBeNull();
    expect(selectBindPayload([] as Binds, 3)).toBeNull();
  });
});
