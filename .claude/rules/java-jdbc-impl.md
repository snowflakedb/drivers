# JDBC source implementation

Apply when editing `jdbc/src/main/java`. Arctic Owl source of truth:
`.ai/review/universal-driver-java-impl.yaml`. Test files stay under
`universal-driver-java.yaml` and the `jdbc-test-reviewer` skill.

## Boundary

- Impl methods throw runtime carriers (`SFSQLException`, `CoreException`,
  `NotImplementedException`). Do not add `throws SQLException` on impls.
- Checked `SQLException` is created only in `SqlExceptionMapper` / generated
  `@JdbcBoundary` decorators. Constructors that run before decoration use
  `SqlExceptionMapper.call`.
- Hand JDBC objects out through `Decorators.*`. Do not return a raw `*Impl`
  across the public API.

## Seams and concurrency

- No new `public static` mutable fields as test/source overrides. Inject
  collaborators such as `CoreDriverApi` (constructor injection is typical;
  `SnowflakeConnectionImpl` is an example site, not the only one).
- No static `SimpleDateFormat` / `DateFormat` / `Calendar`. Use
  `DateTimeFormatter` or a per-call clone.
- Closed-handle flags are `volatile` or `AtomicBoolean`. `checkClosed()`
  throws `SFSQLException` with `CONNECTION_CLOSED` / `RESULTSET_ALREADY_CLOSED`.

## Errors and resources

- Catch the types the callee throws. Do not collapse unexpected failures into
  a generic invalid/false result.
- `NotImplementedException` / `SFSQLFeatureNotSupportedException` must name
  the method.
- Arrow `RootAllocator` must be bounded. Native/Arrow buffers close on every
  path (`addSuppressed` when closing several).

## Growth

- New JDBC surface goes in a package-private collaborator or the decorator,
  not another 50 lines on `Snowflake*Impl` / `MetaDataObjects`.
- Do not rip out `@JdbcBoundary`, `SqlExceptionMapper`, or `DelegatingWrapper`.

<!-- sync-target: .cursor/rules/java-jdbc-impl.mdc carries this body plus Cursor
     frontmatter. This rule is glob-scoped (not alwaysApply), so a pointer would
     work, but keep full content so the glob match is self-contained.
     TO UPDATE: edit this file, copy it below the .mdc frontmatter, then run
     bash scripts/check-ai-rules-sync.sh -->
