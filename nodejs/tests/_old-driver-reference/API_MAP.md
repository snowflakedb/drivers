# Snowflake SDK API Map

A tree-like map of the public API surface exposed by the `snowflake-sdk` package
(`snowflake-connector-nodejs`, version `2.4.1`).

The map is derived from:

- The TypeScript declarations in `index.d.ts` (public, documented surface)
- The runtime entrypoint `index.js` → `lib/snowflake.ts` → `lib/core.js`
- The implementation files for `Connection`, `Statement`, and `Column`

Methods marked **[runtime-only]** are exported by the implementation but are
**not** declared in `index.d.ts`. They are reachable from JavaScript code and
visible at runtime, but TypeScript users will need a cast or `// @ts-expect-error`
to call them. The marker is informational, not a recommendation — treat them as
internal/unstable unless documented elsewhere.

```text
snowflake-sdk  (default export from index.js)
│
├── Module-level constants
│   ├── STRING                                    // NativeType value, exposed via Object.defineProperties
│   ├── BOOLEAN
│   ├── NUMBER
│   ├── DATE
│   ├── OBJECT
│   ├── ARRAY
│   ├── MAP
│   ├── JSON
│   ├── ocspModes : OcspModes
│   │   ├── FAIL_CLOSED : string
│   │   ├── FAIL_OPEN   : string
│   │   └── INSECURE    : string
│   └── ErrorCode : ErrorCodeEnum                 // numeric enum, see lib/error_code.ts
│
├── Module-level functions
│   ├── createConnection(options? : ConnectionOptions)
│   │      → Connection
│   │      // When called without options, reads ~/.snowflake/connections.toml.
│   │      // Honors SNOWFLAKE_HOME and SNOWFLAKE_DEFAULT_CONNECTION_NAME env vars.
│   │
│   ├── createPool(
│   │      options?     : ConnectionOptions,
│   │      poolOptions? : import('generic-pool').Options,
│   │   ) → Pool<Connection>                      // from generic-pool; see "Pool<Connection>" subtree below
│   │
│   ├── configure(options? : ConfigureOptions) → void
│   │      // Applies global driver configuration (log level, OCSP, parsers, etc.).
│   │
│   ├── serializeConnection(connection : Connection) → string
│   │      // Functional equivalent of Connection.serialize().
│   │
│   ├── deserializeConnection(
│   │      options              : ConnectionOptions,
│   │      serializedConnection : string,
│   │   ) → Connection
│   │      // Rehydrates a connection from a string produced by serialize().
│   │
│   └── normalizeConnectionOptions(
│          options : Record<string, unknown>,
│      ) → ConnectionOptions
│          // snake_case → camelCase + alias resolution
│          // (e.g. user → username, private_key_file → privateKeyPath).
│
├── Connection  (returned by createConnection / deserializeConnection; also a Node.js EventEmitter)
│   ├── Lifecycle
│   │   ├── connect(callback? : ConnectionCallback) → Connection          // delegates to connectAsync()
│   │   ├── connectAsync(callback? : ConnectionCallback) → Promise<Connection>
│   │   └── destroy(callback : ConnectionCallback) → void                 // terminates the session
│   │
│   ├── Health / state
│   │   ├── isUp() → boolean                                              // session is connected
│   │   ├── isValidAsync() → Promise<boolean>                             // up + heartbeat succeeded
│   │   ├── isTokenValid() → boolean                                      // [runtime-only] session+master tokens unexpired
│   │   ├── getId() → string                                              // randomUUID assigned at construction
│   │   ├── getServiceName() → string                                     // SERVICE_NAME parameter value
│   │   ├── getClientSessionKeepAlive() → boolean                         // [runtime-only]
│   │   ├── getClientSessionKeepAliveHeartbeatFrequency() → number        // [runtime-only]
│   │   ├── getJsTreatIntegerAsBigInt() → boolean                         // [runtime-only]
│   │   ├── getTokens() → object                                          // [runtime-only] only returns data in QA mode
│   │   └── determineConnectionDomain() → 'GLOBAL' | 'CHINA'               // [runtime-only]
│   │
│   ├── Keep-alive
│   │   ├── heartbeat(callback? : Function) → void                        // [runtime-only] POST /session/heartbeat
│   │   └── heartbeatAsync() → Promise<[{1: 1}]>                          // [runtime-only] promisified heartbeat
│   │
│   ├── OCSP
│   │   └── setupOcspPrivateLink(host : string) → void
│   │
│   ├── Query execution
│   │   ├── execute(options : StatementOption)
│   │   │      → RowStatement | FileAndStageBindStatement
│   │   ├── fetchResult(options : StatementOption)
│   │   │      → RowStatement | FileAndStageBindStatement
│   │   └── getResultsFromQueryId(
│   │          options : { queryId: string } & Partial<StatementOption>,
│   │       ) → Promise<RowStatement | FileAndStageBindStatement>
│   │
│   ├── Asynchronous query status
│   │   ├── getQueryStatus(queryId : string) → Promise<QueryStatus>
│   │   ├── getQueryStatusThrowIfError(queryId : string) → Promise<QueryStatus>
│   │   ├── isStillRunning(status : QueryStatus) → boolean
│   │   └── isAnError(status : QueryStatus) → boolean
│   │          // NOTE: .d.ts currently types this as isAnError() with no args;
│   │          // the runtime implementation accepts and uses `status`.
│   │
│   ├── Serialization
│   │   └── serialize() → string                                          // JSON string with token info; unstable format
│   │
│   └── EventEmitter API
│       ├── on(event, listener)
│       ├── once(event, listener)
│       ├── emit(event, ...args)
│       └── …all other Node.js EventEmitter methods
│
├── RowStatement  (returned by Connection.execute / fetchResult / getResultsFromQueryId)
│   │  // Also a Node.js EventEmitter ('statement-complete' is emitted on completion).
│   │
│   ├── Identity / metadata
│   │   ├── getSqlText() → string
│   │   ├── getStatus() → StatementStatus              // 'fetching' | 'complete'
│   │   ├── getRequestId() → string
│   │   ├── getQueryId() → string                      // server-assigned query id
│   │   └── getStatementId() → string                  // @deprecated alias for getQueryId
│   │
│   ├── Results metadata
│   │   ├── getColumns() → Column[] | undefined
│   │   ├── getColumn(columnIdentifier : string | number) → Column
│   │   ├── getNumRows() → number
│   │   ├── getNumUpdatedRows() → number | undefined
│   │   └── getSessionState() → object | undefined     // warehouse/db/schema/role at completion time
│   │
│   ├── Row consumption
│   │   ├── streamRows(options? : StreamOptions) → Readable
│   │   └── fetchRows(options? : StreamOptions)  → Readable
│   │       // StreamOptions: { start?, end?, fetchAsString?, each? }
│   │
│   ├── Cancellation
│   │   └── cancel(callback? : StatementCallback) → void
│   │
│   └── Diagnostics (integration testing)
│       ├── getQueryContextCacheSize() → number        // [runtime-only]
│       └── getQueryContextDTOSize()  → number         // [runtime-only]
│
├── FileAndStageBindStatement extends RowStatement
│   │  // Returned by execute() for PUT / GET commands and for stage-bound bulk inserts.
│   ├── hasNext()        → boolean
│   ├── NextResult()     → void
│   └── getFileMetadata() → object                     // [runtime-only] PUT/GET file transfer metadata
│
├── Column  (returned by RowStatement.getColumns / getColumn)
│   ├── Identity
│   │   ├── getName()      → string
│   │   ├── getIndex()     → number
│   │   ├── getId()        → number
│   │   ├── getType()      → string
│   │   ├── getScale()     → number
│   │   ├── getPrecision() → number
│   │   └── isNullable()   → boolean
│   │
│   ├── Type predicates
│   │   ├── isString()        → boolean
│   │   ├── isBinary()        → boolean
│   │   ├── isNumber()        → boolean
│   │   ├── isBoolean()       → boolean
│   │   ├── isDate()          → boolean
│   │   ├── isTime()          → boolean
│   │   ├── isTimestamp()     → boolean
│   │   ├── isTimestampLtz()  → boolean
│   │   ├── isTimestampNtz()  → boolean
│   │   ├── isTimestampTz()   → boolean
│   │   ├── isVariant()       → boolean
│   │   ├── isObject()        → boolean
│   │   ├── isArray()         → boolean
│   │   └── isMap()           → boolean
│   │
│   └── Row value extraction
│       ├── getRowValue(row : object)         → any
│       └── getRowValueAsString(row : object) → string
│
├── Pool<Connection>  (returned by createPool; from `generic-pool`)
│   │  // Snowflake doesn't subclass the pool; you get the full generic-pool surface.
│   ├── acquire()                       → Promise<Connection>
│   ├── release(connection)             → Promise<void>
│   ├── destroy(connection)             → Promise<void>
│   ├── use(fn)                         → Promise<T>           // acquire → fn → release
│   ├── drain()                         → Promise<void>
│   ├── clear()                         → Promise<void>
│   ├── on('factoryCreateError', err)                          // wired up by createPool to reject waiters
│   └── …other generic-pool methods/properties (size, available, borrowed, …)
│
└── TypeScript types (declaration-only; no runtime presence)
    │
    ├── Interfaces
    │   ├── ConnectionOptions       // union of WIP_ConnectionOptions + legacy/Node-specific options
    │   ├── ConfigureOptions        // GlobalConfigOptionsTyped + logger/OCSP/parser knobs
    │   ├── StatementOption
    │   ├── StreamOptions
    │   ├── Connection                                         // (described above)
    │   ├── RowStatement                                       // (described above)
    │   ├── FileAndStageBindStatement                          // (described above)
    │   ├── Column                                             // (described above)
    │   ├── OcspModes
    │   ├── XMlParserConfigOption
    │   ├── SnowflakeError          extends Error
    │   └── SnowflakeErrorExternal  extends Error
    │
    └── Type aliases
        ├── CustomParser           = (rawColumnValue: string) => any
        ├── Bind                   = string | number | boolean | null
        ├── InsertBinds            = readonly Bind[][]
        ├── Binds                  = readonly Bind[] | InsertBinds
        ├── StatementCallback      = (err, stmt, rows?) => void
        ├── ConnectionCallback     = (err, conn) => void
        ├── RowMode                = 'object' | 'array' | 'object_with_renamed_duplicated_columns'
        ├── LogLevel               = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG' | 'TRACE' | 'OFF'
        ├── DataType               = 'String' | 'Boolean' | 'Number' | 'Date' | 'JSON' | 'Buffer'
        ├── QueryStatus            = 'RUNNING' | 'ABORTING' | 'SUCCESS' | 'FAILED_WITH_ERROR'
        │                          | 'ABORTED' | 'QUEUED' | 'FAILED_WITH_INCIDENT' | 'DISCONNECTED'
        │                          | 'RESUMING_WAREHOUSE' | 'QUEUED_REPARING_WAREHOUSE'
        │                          | 'RESTARTED' | 'BLOCKED' | 'NO_DATA'
        └── StatementStatus        = 'fetching' | 'complete'
```

## Quick reference

### Typical lifecycle

```text
snowflake.createConnection(opts)
    │
    ▼
Connection.connectAsync()                 ── or ──>  Connection.connect(cb)
    │
    ▼
Connection.execute({ sqlText, binds, complete, … })
    │
    ▼
RowStatement / FileAndStageBindStatement
    │
    ├── statement.streamRows(opts) ──> Readable stream of rows
    ├── statement.fetchRows(opts)  ──> Readable (invokes each() per row)
    └── statement.cancel(cb)
    │
    ▼
Connection.destroy(cb)
```

### Async query lifecycle

```text
Connection.execute({ sqlText, asyncExec: true })
    │ statement.getQueryId() ─────────────────────► persist queryId
    ▼
Connection.getQueryStatus(queryId)        // poll
Connection.getQueryStatusThrowIfError(queryId)
Connection.isStillRunning(status) / isAnError(status)
    │
    ▼
Connection.getResultsFromQueryId({ queryId, … })
    │
    ▼
RowStatement / FileAndStageBindStatement
```

### Session sharing across processes

```text
[Process A]                              [Process B]
  Connection.serialize()                   snowflake.deserializeConnection(opts, str)
  ──────────► JSON string ──────────►      Connection (reuses session/master tokens)
                                           (no re-login needed)
```

> Caveat: the serialized format is **not stable across driver versions** —
> only use it with the same version, and only with the same driver type.
