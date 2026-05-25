import { describe, it, expect } from 'vitest';
import ColumnType from '../../src/constants/ColumnType';
import ErrorCode from '../../src/constants/ErrorCode';
import { getSnowflakeSDK } from './utils';

describe('SDK Constants', () => {
  const snowflake = getSnowflakeSDK();

  it('exports ocspModes', () => {
    expect(snowflake.ocspModes).toEqual({
      FAIL_CLOSED: 'FAIL_CLOSED',
      FAIL_OPEN: 'FAIL_OPEN',
      INSECURE: 'INSECURE',
    });
  });

  it('exports ErrrorCode', () => {
    expect(snowflake.ErrorCode).toEqual(ErrorCode);
  });

  it('exports column types', () => {
    expect(snowflake.STRING).toEqual(ColumnType.STRING);
    expect(snowflake.BOOLEAN).toEqual(ColumnType.BOOLEAN);
    expect(snowflake.NUMBER).toEqual(ColumnType.NUMBER);
    expect(snowflake.DATE).toEqual(ColumnType.DATE);
    expect(snowflake.OBJECT).toEqual(ColumnType.OBJECT);
    expect(snowflake.ARRAY).toEqual(ColumnType.ARRAY);
    expect(snowflake.MAP).toEqual(ColumnType.MAP);
    expect(snowflake.JSON).toEqual(ColumnType.JSON);
  });
});
