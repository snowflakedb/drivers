import { describe, expect, it } from 'vitest';
import {
  COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN,
  CONVERTER_FACTORIES_BY_COLUMN_TYPE,
} from '../../../src/query-result/cell-mapping.js';

describe('COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN', () => {
  it('should list every column type under exactly one fetchAsString token', () => {
    const columnTypesInTokens = Object.values(COLUMN_TYPES_FOR_FETCH_AS_STRING_TOKEN).flat();
    expect([...columnTypesInTokens].sort()).toEqual(
      Object.keys(CONVERTER_FACTORIES_BY_COLUMN_TYPE).sort(),
    );
  });
});
