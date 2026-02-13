# ADR: Python Query Metadata

## Status

Accepted

## Context

The universal driver's Python connector exposes per-query metadata through three `Cursor` attributes: `description`, `rowcount`, and `sfqid`. These must behave consistently with the old `snowflake-connector-python` to allow existing user code to migrate without changes.

## Decisions

### 1. `ResultMetadata` NamedTuple replaces plain tuples in `description`

| Aspect | Old driver | Universal driver |
|---|---|---|
| Type of each item | `ResultMetadata` (NamedTuple) | `ResultMetadata` (NamedTuple) |
| Import path | `snowflake.connector.cursor.ResultMetadata` | `snowflake.connector.cursor.ResultMetadata` |
| Tuple length | 7 | 7 |
| Named fields | `name`, `type_code`, `display_size`, `internal_size`, `precision`, `scale`, `is_nullable` | Same |

Because `ResultMetadata` is a `NamedTuple`, it is fully tuple-compatible — indexing (`desc[0][1]`), unpacking, and `len()` all work identically to the old driver.

A backward-compatibility alias `ResultMetadataV2 = ResultMetadata` is provided but is **not** exported from `__init__.py`. The old driver also defines this alias.

#### Breaking change: `is_nullable` field name

The old driver's `ResultMetadata` uses the field name `is_nullable`. The universal driver matches this. However, the `description` docstring references `null_ok` (following PEP 249 naming). Code that accesses the value by **index** (`desc[0][6]`) is unaffected; code that accesses it by **name** should use `is_nullable`.

### 2. `type_code` values match the old driver

Type codes are integer constants (e.g. `FIXED = 0`, `REAL = 1`, `TEXT = 2`) defined in `_internal/type_codes.py`. The mapping is identical to the old driver's `snowflake.connector.constants` so that comparisons like `desc[0][1] == 0` continue to work. Unknown types fall back to `TEXT` (2).

### 3. `rowcount` semantics

| Scenario | Old driver | Universal driver |
|---|---|---|
| Before any execute | `None` | `None` |
| SELECT | Row count from server (`total` field) | Same |
| DML (INSERT/UPDATE/DELETE/MERGE) | Sum of affected-row columns from rowset | Same |
| DDL | Server-reported total (`1`) | Same |
| Unavailable | `-1` | `-1` |

No breaking changes. The Rust core uses the constant `ROWCOUNT_UNKNOWN = -1` and sums the same DML column names as the old driver (`number of rows updated`, `number of multi-joined rows updated`, `number of rows deleted`, and columns starting with `number of rows inserted`). The raw `-1` sentinel is passed through to Python when the server does not report a row count, matching the old driver's behavior.

### 4. `sfqid` property

| Aspect | Old driver | Universal driver |
|---|---|---|
| Attribute name | `sfqid` | `sfqid` |
| Before execute | `None` | `None` |
| After execute | Snowflake Query ID (UUID string) | Same |
| Persistence | Survives fetch calls, updates on next execute | Same |

No breaking changes. The property is backed by `execute_result.query_id` from the protobuf response.

### 5. `description` is populated eagerly

Both drivers populate `description` immediately after `execute()` returns, before any rows are fetched. The metadata comes from the `columns` field of the execute response (protobuf `ColumnMetadata` messages). Each column's optional fields (`length`, `byte_length`, `precision`, `scale`) are set to `None` when the server omits them.

`display_size` is only populated for text types (`TEXT`, `VARCHAR`, `CHAR`, `STRING`); all other types report `None`. This matches the old driver.

## Consequences

- Existing code using tuple-index access to `description` items works without changes.
- Code using `ResultMetadata` named-field access works without changes.
- Code comparing `rowcount == -1` for the unknown case works without changes.
- `ResultMetadataV2` alias exists for compatibility but is not part of the public API surface (`__all__`).
