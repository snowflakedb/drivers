# JDBC Java practices

The JDBC wrapper converts host-language JDBC calls into protobuf RPCs. Protocol
behavior lives in `sf_core`. This note is the Java-side contract for
`jdbc/src/main/java`. Review procedure is the `jdbc-impl-reviewer` skill;
enforceable rules are `.ai/review/universal-driver-java-impl.yaml`.

## What it is

Public JDBC types live in `net.snowflake.client.api.*`. Implementations live in
`net.snowflake.client.internal.api.implementation.*` and are marked
`@JdbcBoundary`. A build-time processor emits `Decorated*` wrappers that record
telemetry and translate runtime carriers into checked `SQLException`.

## Layout

- Public API: `jdbc/src/main/java/net/snowflake/client/api/`
- Impl + decorator factory: `.../internal/api/implementation/`
- Carriers: `.../internal/api/implementation/exception/`
- JNI/protobuf facade: `.../internal/unicore/` (`ProtobufApis`, `CoreDriverApiImpl`)
- Processor: `jdbc/decorator-processor/`

## Error boundary

Impl methods throw only runtime carriers (`SFSQLException`, `CoreException`,
`NotImplementedException`, `SFSQLFeatureNotSupportedException`).
`SqlExceptionMapper.translate` is the single place a `SnowflakeSQLException` is
created. `SnowflakeDriver.connect` calls `SqlExceptionMapper.call` because the
connection constructor runs before any decorator exists.

Closed handles use `ErrorCode.CONNECTION_CLOSED` or
`ErrorCode.RESULTSET_ALREADY_CLOSED`, not `IllegalStateException`.

## Injection

Inject collaborators such as `CoreDriverApi` instead of a public mutable static
test override. Constructor injection is the usual pattern — `SnowflakeConnectionImpl`
is one example, not the only site (`*StatementImpl` and result-set serialization
take the same dependency the same way). Apply the rule at every injection site,
including field or factory wiring if that is how a type receives the collaborator.
The production default comes from `ProtobufApis.coreDriverApi()`.

## Gotchas

- `@NoTelemetry` + `translateHot` exist so per-row ResultSet accessors stay off
  the telemetry path. Do not wrap those getters in `AbstractDecorator.call`.
- `DelegatingWrapper` owns `unwrap` / `isWrapperFor` for impls and decorators.
- Logging secrets stay under `universal-driver-logging.yaml`; do not restate
  them here.
- Test conventions stay in the `jdbc-test-reviewer` skill and
  `universal-driver-java.yaml`.
