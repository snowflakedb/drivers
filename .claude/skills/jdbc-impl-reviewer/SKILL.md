---
name: jdbc-impl-reviewer
description: >
  Reviews JDBC source (jdbc/src/main/java) for Effective Java / Clean Code
  issues specific to this wrapper: decorator boundary, carriers, concurrency,
  resources, and god-class growth. Use when the user says 'review jdbc impl',
  'review JDBC source', 'review this Java implementation',
  'jdbc impl review', 'review jdbc/src/main/java', or asks for a Java good-practices
  review of the Snowflake JDBC driver (not tests).
---

# JDBC implementation review

Review JDBC **source** under `jdbc/src/main/java` only. Tests belong to
`jdbc-test-reviewer`. Load `.ai/review/universal-driver-java-impl.yaml` and
`.ai/context/jdbc-java-practices.md` before commenting.

## Workflow

1. If the user names files, use those. Otherwise
   `git diff --name-only origin/main...HEAD -- 'jdbc/src/main/java/**/*.java'`
   (exclude `protobuf_gen` and generated `Decorated*`).
2. Review each file in full. The hotspot examples below are not an exclusive
   checklist — flag the same class of issue anywhere it appears, and report
   other Effective Java / Clean Code / concurrency / resource / design problems
   even when they do not match a bullet. Map a finding to an Arctic Owl rule id
   when one exists.
3. Do not recommend ripping out `@JdbcBoundary`, `SqlExceptionMapper`,
   `Decorators`, or `DelegatingWrapper`.

## Hotspot examples

These illustrate known JDBC failure modes. They do not limit the review.

- **Boundary** — impls throw carriers only; public JDBC objects go through
  `Decorators.*`; constructors use `SqlExceptionMapper.call`.
- **Seams** — no new public mutable statics (`ud-java-no-public-mutable-static-seam`).
- **Concurrency** — examples include static `SimpleDateFormat` and non-volatile
  closed flags; also flag other shared mutable state, races, and unsafe
  publication (`ud-java-no-thread-unsafe-jdk-dateformat`,
  `ud-java-jdbc-closed-uses-carrier`).
- **Exceptions** — specific catches; closed handles use `SFSQLException` +
  ErrorCode; unsupported methods have messages
  (`ud-java-catch-specific-or-translate`, `ud-java-no-empty-not-implemented`).
- **Resources** — bounded Arrow allocators; close aggregates
  (`ud-java-arrow-allocator-must-be-bounded`, `ud-java-resource-close-aggregates`,
  `ud-java-native-close-on-all-paths`).
- **Growth** — new surface in collaborators, not `*Impl` bloat
  (`ud-java-prefer-composition-at-jdbc-surface`).

## Output

Per file, group High / Medium / Low with rule ids, then a short checklist:

```
## jdbc/src/main/java/.../Foo.java

### High
- [ud-java-no-thread-unsafe-jdk-dateformat] static SimpleDateFormat at line N

### Medium
- [ud-java-catch-specific-or-translate] catch (Exception) at line N

## Checklist
- [ ] Carriers only on impls
- [ ] No public mutable static seams
- [ ] Closed-handle SQLSTATE via SFSQLException
- [ ] Bounded allocators / close on all paths
```
