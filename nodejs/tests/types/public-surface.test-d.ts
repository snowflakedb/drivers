import type newSnowflakeSdk from 'snowflake-sdk';
import type oldSnowflakeSdk from 'snowflake-sdk-old';
import { describe, expectTypeOf, it } from 'vitest';

// A user fake: a plain object with only the public members, and no class behind it.
type PlainObjectOf<T> = { [K in keyof T]: T[K] };

describe('Connection', () => {
  type Connection = newSnowflakeSdk.Connection;

  it('should accept a plain object with the same members', () => {
    expectTypeOf<PlainObjectOf<Connection>>().toExtend<Connection>();
  });

  it('should keep every old driver member except the removed ones', () => {
    expectTypeOf<Exclude<keyof oldSnowflakeSdk.Connection, keyof Connection>>().toEqualTypeOf<
      // BD#158
      | 'getServiceName'
      // OCSP checks are removed, CRL checks replace them.
      | 'setupOcspPrivateLink'
    >();
  });
});

describe('RowStatement', () => {
  type RowStatement = newSnowflakeSdk.RowStatement;

  it('should accept a plain object with the same members', () => {
    expectTypeOf<PlainObjectOf<RowStatement>>().toExtend<RowStatement>();
  });

  it('should keep every old driver member except the removed ones', () => {
    expectTypeOf<Exclude<keyof oldSnowflakeSdk.RowStatement, keyof RowStatement>>().toEqualTypeOf<
      // BD#157
      | 'getStatementId'
      // BD#173
      | 'fetchRows'
    >();
  });
});

describe('Column', () => {
  type Column = newSnowflakeSdk.Column;

  it('should accept a plain object with the same members', () => {
    expectTypeOf<PlainObjectOf<Column>>().toExtend<Column>();
  });

  it('should keep every old driver member', () => {
    expectTypeOf<Exclude<keyof oldSnowflakeSdk.Column, keyof Column>>().toEqualTypeOf<never>();
  });
});
