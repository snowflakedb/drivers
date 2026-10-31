import { XMLParser, XMLValidator, X2jOptions } from 'fast-xml-parser';
import ErrorCode from './constants/ErrorCode.js';
import ErrorMessage from './constants/ErrorMessage.js';
import { coreSetCustomCredentialManager } from './core/index.js';
import { invalidParameterError } from './error.js';
import { defaultLogger } from './logging/default-logger.js';
import { parseLogLevel, type LogLevel } from './logging/log-level.js';
import { logger, logRouter, type SnowflakeLogger } from './logging/logger.js';

export type CustomParser = (rawColumnValue: string) => any;
export type XMlParserConfigOption = X2jOptions;

/** Token store; methods may return a value or a Promise that settles within 30 seconds. */
export interface CustomCredentialManager {
  read(key: string): string | null | undefined | Promise<string | null | undefined>;
  write(key: string, token: string): unknown | Promise<unknown>;
  remove(key: string): unknown | Promise<unknown>;
}

export interface ConfigureOptions {
  /**
   * How much the driver logs. Any letter case works.
   *
   * - `OFF`: nothing.
   * - `ERROR`: errors only.
   * - `WARN`: errors and warnings.
   * - `INFO`: errors, warnings and info messages.
   * - `DEBUG`: everything.
   * - `TRACE`: same as `DEBUG`. The driver doesn't produce trace-level logs, but `TRACE` is
   *   kept for backward compatibility with older driver versions.
   *
   * @default 'INFO'
   */
  logLevel?: LogLevel;

  /**
   * Path of the JSON-line log file. Missing parent directories are created, and the file is
   * opened by `configure` so an unusable path fails there. `STDOUT` (any letter case) writes
   * to stdout only.
   *
   * @default 'snowflake.log'
   */
  logFilePath?: string;

  /**
   * Whether log lines also go to stdout when they are written to a file.
   *
   * @default true
   */
  additionalLogToConsole?: boolean | null;

  /**
   * Receives the level-filtered, formatted messages instead of the built-in file and console
   * output. Setting it back to `undefined` restores the built-in output.
   */
  customLogger?: SnowflakeLogger;

  /**
   * Custom parser for JSON data in VARIANT, OBJECT, ARRAY, and MAP columns.
   *
   * The driver uses `JSON.parse()` unless this option is set. `JSON.parse()` fails on `NaN`,
   * `Infinity`, and `undefined`, which Snowflake returns while the `STRICT_JSON_OUTPUT`
   * parameter is `FALSE` (its default value). For example, `ARRAY_CONSTRUCT(1, NULL, 2)`
   * returns `[1, undefined, 2]`. To read such values, either:
   * - set `STRICT_JSON_OUTPUT` to `TRUE`, so Snowflake returns valid JSON.
   * - set this option to a parser that accepts these values.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume
   * @see https://docs.snowflake.com/en/sql-reference/parameters#strict-json-output
   */
  jsonColumnVariantParser?: CustomParser;

  /**
   * Custom parser for XML data in VARIANT columns.
   *
   * The driver always attempts JSON parsing first for every VARIANT value. Only when
   * JSON parsing fails does it try this XML parser, so XML values always incur the
   * overhead of a failed JSON parse attempt before being handled.
   *
   * The built-in parser uses `fast-xml-parser` and ignores XML attributes by default.
   * Use `xmlParserConfig` to customize attribute handling.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume
   */
  xmlColumnVariantParser?: CustomParser;

  /**
   * Configuration passed through to `fast-xml-parser`, the library backing the
   * built-in `xmlColumnVariantParser`.
   *
   * These options are forwarded directly to the `XMLParser` constructor, so they
   * control how XML in VARIANT columns is parsed (e.g. attribute handling, tag value
   * processing). See `fast-xml-parser`'s `X2jOptions` for the full set of options.
   *
   * Has no effect if you supply your own `xmlColumnVariantParser`.
   */
  xmlParserConfig?: XMlParserConfigOption;

  /**
   * Whether the driver reads proxy settings from the `HTTP_PROXY`, `HTTPS_PROXY`, and `NO_PROXY`
   * environment variables when a connection does not configure an explicit proxy. An explicit
   * `proxyHost`/`proxyPort` always takes precedence over the environment.
   *
   * @default true
   */
  useEnvProxy?: boolean;

  /**
   * HTTP request timeout for CRL download, in milliseconds. Applies to connections created after
   * the call. The value is rounded down to whole seconds; a positive value under 1000 becomes 1
   * second.
   *
   * @default 10000
   */
  crlDownloadTimeout?: number;

  /**
   * Maximum allowed size of a CRL download response, in bytes. Applies to connections created
   * after the call.
   *
   * @default 20971520 (20 MB)
   */
  crlDownloadMaxSize?: number;

  /**
   * Time after which cached CRL entries are invalidated, in milliseconds. Applies to connections
   * created after the call. The value is rounded down to whole seconds.
   *
   * @default 86400000 (24 hours)
   */
  crlCacheValidityTime?: number;

  /**
   * Directory for the on-disk CRL cache. Applies to connections created after the call.
   *
   * @default process.env.SNOWFLAKE_CRL_ON_DISK_CACHE_DIR, otherwise the driver's default cache
   * directory
   */
  crlCacheDir?: string;

  /**
   * Token store that replaces the OS keyring for cached credentials.
   *
   * Once registered, it is used by default for MFA, external browser, and OAuth
   * Authorization Code logins, because `clientStoreTemporaryCredential` defaults to `true`.
   * A connection that sets `clientStoreTemporaryCredential: false` never calls it.
   *
   * It receives ID tokens, MFA tokens, OAuth access and refresh tokens, and DPoP private keys.
   */
  customCredentialManager?: CustomCredentialManager;
}

const LOGGING_OPTIONS: (keyof ConfigureOptions)[] = [
  'logLevel',
  'logFilePath',
  'additionalLogToConsole',
  'customLogger',
];

type ResolvedConfig = Required<
  Omit<ConfigureOptions, 'customCredentialManager' | 'crlCacheDir' | 'customLogger'>
> &
  Pick<ConfigureOptions, 'crlCacheDir' | 'customLogger'>;

const buildDefaults = (): ResolvedConfig => ({
  logLevel: 'INFO',
  logFilePath: 'snowflake.log',
  additionalLogToConsole: true,
  customLogger: undefined,
  jsonColumnVariantParser: (rawColumnValue: string) => JSON.parse(rawColumnValue),
  xmlColumnVariantParser: (rawColumnValue: string) => {
    const validateResult = XMLValidator.validate(rawColumnValue);
    if (validateResult === true) {
      return new XMLParser(GlobalConfig.xmlParserConfig).parse(rawColumnValue);
    } else {
      throw new Error(validateResult.err.msg);
    }
  },
  xmlParserConfig: {},
  useEnvProxy: true,
  crlDownloadTimeout: 10000,
  crlDownloadMaxSize: 20 * 1024 * 1024,
  crlCacheValidityTime: 86400000,
  crlCacheDir: process.env.SNOWFLAKE_CRL_ON_DISK_CACHE_DIR,
});

export const GlobalConfig: ResolvedConfig = {
  ...buildDefaults(),
};

export function updateGlobalConfig({ customCredentialManager, ...options }: ConfigureOptions) {
  if (customCredentialManager != null) {
    setCustomCredentialManager(customCredentialManager);
  }
  const logLevel = parseLogLevel(options.logLevel);
  Object.assign(GlobalConfig, options, logLevel && { logLevel });

  if (LOGGING_OPTIONS.some((key) => key in options)) {
    applyLoggingOptions();
  }
}

function applyLoggingOptions() {
  logRouter.syncLevel();
  if (!GlobalConfig.customLogger && GlobalConfig.logLevel !== 'OFF') {
    defaultLogger.openFile();
  }
  logger.info(
    'Configuring logger with level: %s, filePath: %s, additionalLogToConsole: %s, customLogger: %s',
    GlobalConfig.logLevel,
    GlobalConfig.logFilePath,
    GlobalConfig.additionalLogToConsole,
    GlobalConfig.customLogger !== undefined,
  );
}

// TODO:
// - entire thing can live in nodejs_bridge including CustomCredentialManager
// - remove validation and trust TypeScript
function setCustomCredentialManager(manager: CustomCredentialManager) {
  const isValid =
    typeof manager === 'object' &&
    (['read', 'write', 'remove'] as const).every(
      (method) => Object.hasOwn(manager, method) && typeof manager[method] === 'function',
    );
  if (!isValid) {
    const code = ErrorCode.ERR_GLOBAL_CONFIGURE_INVALID_CUSTOM_CREDENTIAL_MANAGER;
    throw invalidParameterError(ErrorMessage[code], code);
  }
  coreSetCustomCredentialManager({
    read: async (key) => manager.read(key),
    write: async (key, token) => {
      await manager.write(key, token);
    },
    remove: async (key) => {
      await manager.remove(key);
    },
  });
}

export const resetGlobalConfig = () => {
  Object.assign(GlobalConfig, buildDefaults());
  coreSetCustomCredentialManager(null);
  defaultLogger.close();
  logRouter.syncLevel();
};
