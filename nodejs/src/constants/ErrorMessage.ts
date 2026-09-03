const ErrorMessage: { [code: number]: string } = {
  // 400001
  400001: 'An internal error has occurred. Please contact Snowflake support.',

  // 401001
  401001: 'Network error. Could not reach Snowflake.',
  401002: 'Request to Snowflake failed.',
  401003: 'Snowflake responded with non-JSON content.',
  401004: 'Request to Snowflake failed.  Invalid token',

  // 402001
  402001: 'Network error. Could not reach S3/Blob.',
  402002: 'Request to S3/Blob failed.',

  // 403001
  403001:
    'Invalid logLevel. The specified value must be one of these five levels: error, warn, debug, info and trace.',
  403002: 'Invalid disableOCSPChecks option. The specified value must be a boolean.',
  403003:
    'Invalid OCSP mode. The specified value must be FAIL_CLOSED, FAIL_OPEN, or INSECURE_MODE.',
  403004: 'Invalid custom JSON parser. The specified value must be a function.',
  403005: 'Invalid custom XML parser. The specified value must be a function.',
  403006: 'Invalid keep alive value. The specified value must be a boolean.',
  403007:
    'Invalid custom credential manager value. The specified value must be an object, and it should have three methods: write, read, remove',
  403008: 'Invalid useEnvProxy value. The specified value must be a boolean.',

  // 404001
  404001: 'Connection options must be specified.',
  404002: 'Invalid connection options. The specified value must be an object.',
  404003: 'A user name must be specified.',
  404004: 'Invalid user name. The specified value must be a string.',
  404005: 'A password must be specified.',
  404006: 'Invalid password. The specified value must be a string.',
  404007: 'An account must be specified.',
  404008: 'Invalid account. The specified value must be a string.',
  404009: 'An accessUrl must be specified.',
  404010: 'Invalid accessUrl. The specified value must be a string.',
  404011: 'Invalid warehouse. The specified value must be a string.',
  404012: 'Invalid database. The specified value must be a string.',
  404013: 'Invalid schema. The specified value must be a string.',
  404014: 'Invalid role. The specified value must be a string.',
  404015: 'A proxyHost must be specified',
  404016: 'Invalid proxyHost. The specified value must be a string.',
  404017: 'A proxyPort must be specified.',
  404018: 'Invalid proxyPort. The specified value must be a number.',
  404019: 'Invalid streamResult flag. The specified value must be a boolean.',
  404020: 'Invalid fetchAsString option. The specified value must be an Array.',
  404021:
    'Invalid fetchAsString type: %s. The supported types are: String, Boolean, Number, Date, Buffer, and JSON.',
  404022: 'Invalid region. The specified value must be a string.',
  404023: 'Invalid clientSessionKeepAlive. The specified value must be a boolean.',
  404024: 'Invalid clientSessionKeepAliveHeartbeatFrequency. The specified value must be a number.',
  404025: 'Invalid jsTreatIntegerAsBigInt. The specified value must be a boolean',
  404026: 'Invalid private key. The specified value must be a string in pem format of type pkcs8',
  404027: 'Invalid private key file location. The specified value must be a string',
  404028: 'Invalid private key passphrase. The specified value must be a string',
  404029: 'Invalid oauth token. The specified value must be a string',
  404030: 'Invalid validate default parameters value. The specified value must be a boolean',
  404031:
    'Invalid application value. The specified value must be a string that starts with a letter and a length between 1-50',
  404032: 'A proxyUser must be specified',
  404033: 'Invalid proxyUser. The specified value must be a string.',
  404034: 'A proxyPassword must be specified.',
  404035: 'Invalid proxyPassword. The specified value must be a string.',
  404036: 'Invalid noProxy. The specified value must be a string.',
  404037: 'Invalid arrayBindingThreshold. The specified value must be a number.',
  404038: 'Invalid gcsUseDownscopedCredential. The specified value must be a boolean.',
  404039: 'Invalid forceStageBindError. The specified value must be a number.',
  404040: 'Invalid browser timeout value. The specified value must be a positive number.',
  404041: 'Invalid disableQueryContextCache. The specified value must be a boolean.',
  404042: 'Invalid includeRetryReason. The specified value must be a boolean.',
  404043: 'Invalid clientConfigFile value. The specified value must be a string.',
  404044: 'Invalid retryTimeout value. The specified value must be a number.',
  404045: 'Invalid account. The specified value must be a valid subdomain string.',
  404046: 'Invalid region. The specified value must be a valid subdomain string.',
  404047: 'Invalid disableConsoleLogin. The specified value must be a boolean',
  404049: 'Invalid clientStoreTemporaryCredential. The specified value must be a boolean.',
  404050: 'Invalid representNullAsStringNull. The specified value must be a boolean',
  404051: 'Invalid disableSamlURLCheck. The specified value must be a boolean',
  404052: 'Invalid clientRequestMFAToken. The specified value must be a boolean.',
  404054: 'Invalid host. The specified value must be a string.',
  404055: 'Invalid passcodeInPassword. The specified value must be a boolean',
  404056: 'Invalid passcode. The specified value must be a string',
  404057: 'A password or token must be specified.',
  404058:
    'Invalid oauth authorization URL. The specified value must be a valid URL starting with the https or http protocol.',
  404059: 'Invalid oauth client id. The specified value must not be an empty string',
  404060: 'Invalid oauth client secret. The specified value must not be an empty string',
  404061:
    'Invalid oauth token request URL. The specified value must be a valid URL starting with the https or http protocol.',
  404062: 'Invalid authenticator: WORKLOAD_IDENTITY parameters. %s',
  404063: 'Invalid query tag. The specified value must be a string;',

  // 405001
  405001: 'Invalid callback. The specified value must be a function.',

  // 405501
  405501: 'Connection already in progress.',
  405502: 'Already connected.',
  405503: 'Connection already terminated. Cannot connect again.',
  405505: 'Configuration from client config file failed',
  405506: 'Wrong authorization type',
  405507: 'Authenticator not allowed',

  // 406001
  406001: 'Invalid callback. The specified value must be a function.',

  // 406501
  406501: 'Not connected, so nothing to destroy.',
  406502: 'Already disconnected.',

  // 407001
  407001: 'Unable to perform operation because a connection was never established.',
  407002: 'Unable to perform operation using terminated connection.',

  // 408001
  408001: 'A serializedConnection must be specified.',
  408002: 'Invalid serializedConnection. The specified value must be a string.',
  408003:
    "Invalid serializedConnection. The value must be a string obtained by calling another connection's serialize() method.",

  // 409001
  409001: 'Execute options must be specified.',
  409002: 'Invalid execute options. The specified value must be an object.',
  409003: 'A sqlText value must be specified.',
  409004: 'Invalid sqlText. The specified value must be a string.',
  409005: 'Invalid internal flag. The specified value must be a boolean.',
  409006: 'Invalid parameters. The specified value must be an object.',
  409007: 'Invalid binds. The specified value must be an array.',
  409008: 'Invalid bind variable: %s. Only stringifiable values are supported.',
  409009: 'Invalid complete callback. The specified value must be a function.',
  409010: 'Invalid streamResult flag. The specified value must be a boolean.',
  409011: 'Invalid fetchAsString value. The specified value must be an Array.',
  409012:
    'Invalid fetchAsString type: %s. The supported types are: String, Boolean, Number, Date, Buffer, and JSON.',
  409013: 'Invalid requestId. The specified value must be a string.',
  409014: 'Invalid asyncExec. The specified value must be a boolean.',
  409015: 'Invalid describeOnly. The specified value must be a boolean.',

  // 410001
  410001: 'Fetch-result options must be specified.',
  410002: 'Invalid options. The specified value must be an object.',
  410003: 'A query id/statement id must be specified.',
  410004: 'Invalid query id/statement id. The specified value must be a string.',
  410005: 'Invalid complete callback. The specified value must be a function.',
  410006: 'Invalid streamResult flag. The specified value must be a boolean.',
  410007: 'Invalid fetchAsString value. The specified value must be an Array.',
  410008:
    'Invalid fetchAsString type: %s. The supported types are: String, Boolean, Number, Date, Buffer, and JSON.',
  410009: 'Invalid cwd (current working directory) type: %s. The specified value must be a string.',

  // 411001
  411001: 'Invalid options. The specified value must be an object.',
  411002: 'Invalid start index. The specified value must be a number.',
  411003: 'Invalid end index. The specified value must be a number.',
  411004: 'Invalid fetchAsString value. The specified value must be an Array.',
  411005:
    'Invalid fetchAsString type: %s. The supported types are: String, Boolean, Number, Date, Buffer, and JSON.',
  411006:
    'Invalid row mode value. The specified value should be array or object or object_with_renamed_duplicated_columns',

  412001: 'Certificate is REVOKED.',
  412002: 'Certificate status is UNKNOWN.',
  412003: 'Not recognize signature algorithm.',
  412004: 'Invalid signature.',
  412005: 'No OCSP response data is attached.',
  412006: 'Invalid validity.',
  412007: 'Could not verify the certificate revocation status.',
  412008: 'Not two elements are in the cache.',
  412009: 'Cache entry expired.',
  412010: 'Failed to parse OCSP response.',
  412011: 'Invalid Signing Certificate validity.',
  412012: 'Timeout OCSP responder.',
  412013: 'Timeout OCSP Cache server.',
  412014: 'Failed to obtain OCSP response: %s',
  412015: 'The OCSP response does not correspond to the certificate being checked.',

  413001: 'CRL validation failed.',

  // 450001
  450001: 'Fetch-row options must be specified.',
  450002: 'Invalid options. The specified value must be an object.',
  450003: 'An each() callback must be specified.',
  450004: 'Invalid each() callback. The specified value must be a function.',
  450005: 'An end() callback must be specified.',
  450006: 'Invalid end() callback. The specified value must be a function.',
  450007: 'Operation failed because the statement is still in progress.',

  // 460001
  460001: 'Invalid queryId: %s',
  460002: 'Cannot retrieve data. No information returned from server for query %s',
  460003: 'Status of query %s is %s, results are unavailable',
};

export default ErrorMessage;
