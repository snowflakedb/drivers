import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import * as importedSdk from 'snowflake-sdk';
import ts from 'typescript';
import { beforeAll, describe, expect, it, onTestFinished } from 'vitest';
import { IS_RUNNING_FOR_OLD_DRIVER } from './utils/index.js';

describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('Package ESM vs CommonJS shape', () => {
  const requiredSdk = createRequire(import.meta.url)('snowflake-sdk');

  it('should return the required object as the ESM default export', () => {
    expect(importedSdk.default).toBe(requiredSdk);
  });

  it('should expose named ESM exports', () => {
    // A few exports are enough to check that ESM and require share the same objects.
    // The 'Package export list' tests below check every export.
    expect(importedSdk.createConnection).toBe(requiredSdk.createConnection);
    expect(importedSdk.SnowflakeDate).toBe(requiredSdk.SnowflakeDate);
    expect(importedSdk.ErrorCode).toBe(requiredSdk.ErrorCode);
  });

  it('should expose every ErrorCode member with its reverse mapping', () => {
    expect({ ...requiredSdk.ErrorCode }).toEqual({
      ERR_SF_NETWORK_COULD_NOT_CONNECT: 401001,
      ERR_SF_RESPONSE_FAILURE: 401002,
      ERR_LARGE_RESULT_SET_NETWORK_COULD_NOT_CONNECT: 402001,
      ERR_LARGE_RESULT_SET_RESPONSE_FAILURE: 402002,
      ERR_CONN_CONNECT_STATUS_CONNECTING: 405501,
      ERR_CONN_CONNECT_STATUS_CONNECTED: 405502,
      ERR_CONN_CONNECT_STATUS_DISCONNECTED: 405503,
      ERR_CONN_DESTROY_STATUS_PRISTINE: 406501,
      ERR_CONN_DESTROY_STATUS_DISCONNECTED: 406502,
      ERR_CONN_REQUEST_STATUS_PRISTINE: 407001,
      ERR_CONN_REQUEST_STATUS_DISCONNECTED: 407002,
      ERR_GET_RESULTS_QUERY_ID_NO_DATA: 460002,
      ERR_GET_RESULTS_QUERY_ID_NOT_SUCCESS_STATUS: 460003,
      401001: 'ERR_SF_NETWORK_COULD_NOT_CONNECT',
      401002: 'ERR_SF_RESPONSE_FAILURE',
      402001: 'ERR_LARGE_RESULT_SET_NETWORK_COULD_NOT_CONNECT',
      402002: 'ERR_LARGE_RESULT_SET_RESPONSE_FAILURE',
      405501: 'ERR_CONN_CONNECT_STATUS_CONNECTING',
      405502: 'ERR_CONN_CONNECT_STATUS_CONNECTED',
      405503: 'ERR_CONN_CONNECT_STATUS_DISCONNECTED',
      406501: 'ERR_CONN_DESTROY_STATUS_PRISTINE',
      406502: 'ERR_CONN_DESTROY_STATUS_DISCONNECTED',
      407001: 'ERR_CONN_REQUEST_STATUS_PRISTINE',
      407002: 'ERR_CONN_REQUEST_STATUS_DISCONNECTED',
      460002: 'ERR_GET_RESULTS_QUERY_ID_NO_DATA',
      460003: 'ERR_GET_RESULTS_QUERY_ID_NOT_SUCCESS_STATUS',
    });
  });

  it('should point the default property back to the required object', () => {
    expect(requiredSdk.default).toBe(requiredSdk);
  });

  // Users stub driver functions in their own tests, as they could with the old driver.
  // This breaks if the exports become read-only getters (as `export { X } from` compiles to),
  // or if `.default` is a copy that does not see the stub.
  it('should allow stubbing a method through require', () => {
    const original = requiredSdk.createConnection;
    onTestFinished(() => {
      requiredSdk.createConnection = original;
    });

    const stub = () => ({});
    requiredSdk.createConnection = stub;
    expect(requiredSdk.createConnection).toBe(stub);
    expect(requiredSdk.default.createConnection).toBe(stub);
  });
});

// These tests use the TypeScript compiler API, because TypeScript itself can not list
// type-only exports, like `ConnectionOptions`. In a type test, `typeof` sees only runtime
// exports, like `createConnection`. So a type test can not notice that
// `snowflake.ConnectionOptions` is missing from the default export.
//
// TODO: these tests are work in progress and do not protect users yet.
// Today they check only that the default export and the named exports have the same names.
// They do not compare with the old driver, so a change that breaks user code still passes.
// Missing checks:
// - Each old driver export and type exists here with the same signature.
//   Some old features are removed on purpose, for example OCSP
//   (`OcspModes`, `ocspFailOpen`), so the check needs a list of known differences.
//   tests/types/public-surface.test-d.ts compares the member names of `Connection`,
//   `RowStatement` and `Column`, but not their signatures.
// - The published `.d.ts` files are checked. These tests read `src/index.ts`.
describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('Package export list', () => {
  const INDEX_PATH = fileURLToPath(new URL('../../src/index.ts', import.meta.url));
  const TSCONFIG_PATH = fileURLToPath(new URL('../../tsconfig.json', import.meta.url));

  let typeChecker: ts.TypeChecker;
  let indexModule: ts.Symbol;
  let namedExports: ts.Symbol[];
  let defaultExport: ts.Symbol;

  // `export import X = ...`, `export type { X } from` and `export default x` create an alias symbol.
  // It points to the real declaration, for example the class.
  // `getAliasedSymbol` fails on symbols that are not aliases, so those are returned as is.
  const followExportAlias = (symbol: ts.Symbol) =>
    symbol.flags & ts.SymbolFlags.Alias ? typeChecker.getAliasedSymbol(symbol) : symbol;

  beforeAll(() => {
    const tsconfig = ts.getParsedCommandLineOfConfigFile(
      TSCONFIG_PATH,
      {},
      {
        ...ts.sys,
        onUnRecoverableConfigFileDiagnostic: (diagnostic) => {
          throw new Error(ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n'));
        },
      },
    );
    const program = ts.createProgram([INDEX_PATH], tsconfig!.options);

    typeChecker = program.getTypeChecker();
    indexModule = typeChecker.getSymbolAtLocation(program.getSourceFile(INDEX_PATH)!)!;
    const allExports = typeChecker.getExportsOfModule(indexModule);

    namedExports = allExports.filter((symbol) => symbol.name !== 'default');
    defaultExport = followExportAlias(allExports.find((symbol) => symbol.name === 'default')!);
  });

  it('should expose the expected runtime exports by name and on the default export', () => {
    const toDeclarationNames = (symbols: ts.Symbol[]) =>
      Object.fromEntries(
        symbols
          .filter((symbol) => symbol.name !== 'default')
          .map((symbol) => [
            symbol.name,
            typeChecker.getFullyQualifiedName(followExportAlias(symbol)),
          ]),
      );
    const namedRuntimeExports = toDeclarationNames(
      typeChecker.getTypeOfSymbol(indexModule).getProperties(),
    );
    const defaultRuntimeExports = toDeclarationNames(
      typeChecker.getTypeOfSymbol(defaultExport).getProperties(),
    );

    expect(Object.keys(namedRuntimeExports).sort()).toEqual([
      'ErrorCode',
      'SnowflakeDate',
      'configure',
      'createConnection',
      'createPool',
      'deserializeConnection',
      'getFipsStatus',
      'normalizeConnectionOptions',
      'serializeConnection',
    ]);
    expect(defaultRuntimeExports).toEqual(namedRuntimeExports);
  });

  it('should expose the expected types by name and on the default export', () => {
    const namedExportTypes = Object.fromEntries(
      namedExports
        .filter((symbol) => followExportAlias(symbol).flags & ts.SymbolFlags.Type)
        .map((symbol) => [
          symbol.name,
          typeChecker.getFullyQualifiedName(followExportAlias(symbol)),
        ]),
    );
    const defaultExportTypes = Object.fromEntries(
      [...(defaultExport.exports?.values() ?? [])].map((namespaceType) => {
        const typeAlias = namespaceType.declarations!.find(ts.isTypeAliasDeclaration)!;
        const aliasedType = typeAlias.type as ts.TypeReferenceNode;
        const aliasedSymbol = typeChecker.getSymbolAtLocation(aliasedType.typeName)!;
        return [
          namespaceType.name,
          typeChecker.getFullyQualifiedName(followExportAlias(aliasedSymbol)),
        ];
      }),
    );

    expect(Object.keys(namedExportTypes).sort()).toEqual([
      'Bind',
      'Binds',
      'Column',
      'ConfigureOptions',
      'Connection',
      'ConnectionCallback',
      'ConnectionOptions',
      'CustomCredentialManager',
      'CustomParser',
      'DataType',
      'ErrorCode',
      'FetchResultOptions',
      'FipsStatus',
      'InsertBinds',
      'LogLevel',
      'Pool',
      'PoolOptions',
      'QueryStatus',
      'RowMode',
      'RowStatement',
      'SessionState',
      'SnowflakeDate',
      'SnowflakeError',
      'SnowflakeLogger',
      'StatementCallback',
      'StatementOption',
      'StatementStatus',
      'StreamOptions',
      'XMlParserConfigOption',
    ]);
    expect(defaultExportTypes).toEqual(namedExportTypes);
  });

  it('should let every exported class and enum be used as a type', () => {
    const classesAndEnums = namedExports.filter((symbol) => {
      const valueType = typeChecker.getTypeOfSymbol(followExportAlias(symbol)).getSymbol();
      return valueType && valueType.flags & (ts.SymbolFlags.Class | ts.SymbolFlags.Enum);
    });
    const classAndEnumNames = classesAndEnums.map((symbol) => symbol.name);
    const classAndEnumNamesUsableAsTypes = classesAndEnums
      .filter((symbol) => followExportAlias(symbol).flags & ts.SymbolFlags.Type)
      .map((symbol) => symbol.name);

    expect([...classAndEnumNames].sort()).toEqual(['ErrorCode', 'Pool', 'SnowflakeDate']);
    expect(classAndEnumNamesUsableAsTypes).toEqual(classAndEnumNames);
  });
});
