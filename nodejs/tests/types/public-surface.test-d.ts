import type newSnowflakeSdk from 'snowflake-sdk';
import type oldSnowflakeSdk from 'snowflake-sdk-old';
import { describe, expectTypeOf, it } from 'vitest';

// A user fake: a plain object with only the public members, and no class behind it.
type PlainObjectOf<T> = { [K in keyof T]: T[K] };

// Options in both drivers where the new type rejects a value the old type accepts.
// Compatible options map to `never`, so only incompatible names remain in the union.
type IncompatibleOptions<Old, New> = {
  [K in keyof Old & keyof New]: Exclude<Old[K], undefined> extends Exclude<New[K], undefined>
    ? never
    : K;
}[keyof Old & keyof New];

describe('ConfigureOptions', () => {
  type OldOptions = oldSnowflakeSdk.ConfigureOptions;
  type NewOptions = newSnowflakeSdk.ConfigureOptions;

  it('should keep every old driver option except the removed ones', () => {
    expectTypeOf<Exclude<keyof OldOptions, keyof NewOptions>>().toEqualTypeOf<
      // OCSP checks are removed, CRL checks replace them.
      | 'disableOCSPChecks'
      // OCSP checks are removed, CRL checks replace them.
      | 'ocspFailOpen'
      // BD#161
      | 'enableExperimentalMultipartUploads'
      // TODO: No behavior difference describes this removal yet.
      | 'keepAlive'
    >();
  });

  it('should add only the listed options', () => {
    expectTypeOf<Exclude<keyof NewOptions, keyof OldOptions>>().toEqualTypeOf<never>();
  });

  it('should accept every value of a shared option that the old driver accepted', () => {
    expectTypeOf<IncompatibleOptions<OldOptions, NewOptions>>().toEqualTypeOf<
      // BD#165
      | 'customCredentialManager'
      // BD#162
      | 'xmlParserConfig'
    >();
  });
});

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

describe('ConnectionOptions', () => {
  type OldOptions = oldSnowflakeSdk.ConnectionOptions;
  type NewOptions = newSnowflakeSdk.ConnectionOptions;

  it('should keep every old driver option except the removed ones', () => {
    expectTypeOf<Exclude<keyof OldOptions, keyof NewOptions>>().toEqualTypeOf<
      // BD#159
      | 'clientConfigFile'
      // BD#117
      | 'clientRequestMFAToken'
      // BD#121
      | 'credentialCacheDir'
      // BD#63
      | 'disableConsoleLogin'
      // BD#176
      | 'forceStageBindError'
      // BD#181
      | 'gcsUseDownscopedCredential'
      // BD#179
      | 'includeRetryReason'
      // BD#177
      | 'oauthChallengeMethod'
      // BD#141
      | 'oauthHttpAllowed'
      // BD#160
      | 'region'
      // BD#158
      | 'serviceName'
      // BD#90
      | 'sfRetryMaxLoginRetries'
      // OCSP checks are removed, CRL checks replace them.
      | 'useConnectionConfigProxyForOCSP'
      // BD#178
      | 'validateDefaultParameters'
    >();
  });

  it('should add only the listed options', () => {
    expectTypeOf<Exclude<keyof NewOptions, keyof OldOptions>>().toEqualTypeOf<
      // BD#103
      | 'connectionName'
      // BD#154
      | 'jwtTimeout'
      // BD#64
      | 'port'
      // BD#64
      | 'protocol'
      // BD#166
      | 'proxy'
    >();
  });

  it('should accept every value of a shared option that the old driver accepted', () => {
    expectTypeOf<IncompatibleOptions<OldOptions, NewOptions>>().toEqualTypeOf<never>();
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

describe('StatementOption', () => {
  type OldOptions = oldSnowflakeSdk.StatementOption;
  type NewOptions = newSnowflakeSdk.StatementOption;

  it('should keep every old driver option except the removed ones', () => {
    expectTypeOf<Exclude<keyof OldOptions, keyof NewOptions>>().toEqualTypeOf<
      // BD#156
      | 'excludeGuid'
      // BD#156
      | 'queryId'
    >();
  });

  it('should add only the listed options', () => {
    expectTypeOf<Exclude<keyof NewOptions, keyof OldOptions>>().toEqualTypeOf<never>();
  });

  it('should accept every value of a shared option that the old driver accepted', () => {
    // The statement passed to `complete` has neither getStatementId() (BD#157)
    // nor fetchRows() (BD#173).
    expectTypeOf<IncompatibleOptions<OldOptions, NewOptions>>().toEqualTypeOf<'complete'>();
  });
});

describe('StreamOptions', () => {
  type OldOptions = oldSnowflakeSdk.StreamOptions;
  type NewOptions = newSnowflakeSdk.StreamOptions;

  it('should keep every old driver option except the removed ones', () => {
    // BD#173
    expectTypeOf<Exclude<keyof OldOptions, keyof NewOptions>>().toEqualTypeOf<'each'>();
  });

  it('should add only the listed options', () => {
    expectTypeOf<Exclude<keyof NewOptions, keyof OldOptions>>().toEqualTypeOf<never>();
  });

  it('should accept every value of a shared option that the old driver accepted', () => {
    expectTypeOf<IncompatibleOptions<OldOptions, NewOptions>>().toEqualTypeOf<never>();
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
