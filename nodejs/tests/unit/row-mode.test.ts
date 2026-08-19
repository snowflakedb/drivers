import { describe, it, expect } from 'vitest';
import {
  renameDuplicateColumnNames,
  resolveColumnNames,
  reshapeRowForMode,
  DEFAULT_ROW_MODE,
} from '../../src/query-result/row-mode';

describe('renameDuplicateColumnNames', () => {
  const cases: Array<{ name: string; input: string[]; expected: string[] }> = [
    {
      name: 'without duplicates',
      input: ['COL1', 'COL2'],
      expected: ['COL1', 'COL2'],
    },
    {
      name: 'single duplicated column',
      input: ['COL1', 'COL1'],
      expected: ['COL1', 'COL1_2'],
    },
    {
      name: 'empty column list',
      input: [],
      expected: [],
    },
    {
      name: 'skips a suffix that already exists in the input',
      input: ['COL1', 'COL1', 'COL1_2'],
      expected: ['COL1', 'COL1_3', 'COL1_2'],
    },
    {
      name: 'multiple duplicated columns',
      input: ['COL1', 'COL1', 'COL2', 'COL2', 'COL2', 'COL3', 'COL3'],
      expected: ['COL1', 'COL1_2', 'COL2', 'COL2_2', 'COL2_3', 'COL3', 'COL3_2'],
    },
    {
      name: 'multiple duplicated columns despite numeric suffixes already present',
      input: ['COL1', 'COL1_2', 'COL1_2', 'COL1'],
      expected: ['COL1', 'COL1_2', 'COL1_2_2', 'COL1_3'],
    },
    {
      name: 'empty-string names are never renamed, even if repeated',
      input: ['', ''],
      expected: ['', ''],
    },
    {
      name: 'chained renames across repeated and pre-suffixed names',
      input: ['KEY', 'FOO', 'KEY_1', 'FOO', 'KEY_3', 'FOO_3', 'KEY', 'FOO', 'KEY', 'FOO'],
      expected: [
        'KEY',
        'FOO',
        'KEY_1',
        'FOO_2',
        'KEY_3',
        'FOO_3',
        'KEY_2',
        'FOO_4',
        'KEY_4',
        'FOO_5',
      ],
    },
  ];

  for (const { name, input, expected } of cases) {
    it(name, () => {
      expect(renameDuplicateColumnNames(input)).toEqual(expected);
    });
  }

  it('does not mutate its input', () => {
    const input = ['COL1', 'COL1'];
    renameDuplicateColumnNames(input);
    expect(input).toEqual(['COL1', 'COL1']);
  });
});

describe('resolveColumnNames', () => {
  it('renames duplicates only for object_with_renamed_duplicated_columns', () => {
    expect(resolveColumnNames(['A', 'A'], 'object_with_renamed_duplicated_columns')).toEqual([
      'A',
      'A_2',
    ]);
  });

  it('leaves names unchanged for array mode', () => {
    expect(resolveColumnNames(['A', 'A'], 'array')).toEqual(['A', 'A']);
  });

  it('leaves names unchanged for object mode', () => {
    expect(resolveColumnNames(['A', 'A'], 'object')).toEqual(['A', 'A']);
  });
});

describe('reshapeRowForMode', () => {
  const row = [1, 'two', true];
  const columnNames = ['A', 'B', 'C'];

  it('passes the row through unchanged for array mode', () => {
    expect(reshapeRowForMode(row, columnNames, 'array')).toBe(row);
  });

  it('keys the row by column name for object mode', () => {
    expect(reshapeRowForMode(row, columnNames, 'object')).toEqual({ A: 1, B: 'two', C: true });
  });

  it('keys the row by the already-resolved column names for object_with_renamed_duplicated_columns', () => {
    expect(
      reshapeRowForMode(row, ['A', 'A_2', 'C'], 'object_with_renamed_duplicated_columns'),
    ).toEqual({
      A: 1,
      A_2: 'two',
      C: true,
    });
  });

  it('last value wins when two columns share a name (plain object mode)', () => {
    expect(reshapeRowForMode([1, 2], ['DUP', 'DUP'], 'object')).toEqual({ DUP: 2 });
  });
});

describe('DEFAULT_ROW_MODE', () => {
  it('is object, matching the old driver default', () => {
    expect(DEFAULT_ROW_MODE).toBe('object');
  });
});
