# TIMESTAMP_NTZ Design Document

## What TIMESTAMP_NTZ Is

TIMESTAMP_NTZ (No Time Zone) stores a wall-clock datetime without any timezone information. Snowflake preserves the literal value as entered — it does not adjust for session timezone, UTC, or any offset. The session `TIMEZONE` parameter has no effect on TIMESTAMP_NTZ values.

**Synonyms:** `TIMESTAMP` and `DATETIME` are aliases for `TIMESTAMP_NTZ` by default (when `TIMESTAMP_TYPE_MAPPING = TIMESTAMP_NTZ`).

**Precision:** Snowflake supports nanosecond precision internally (scale 0–9). The Python driver returns microsecond precision because Python `datetime` is capped at 6 decimal places. Arrow data carries a per-column scale in the schema metadata (0–9). The driver unconditionally initialises `scale = 9` at `ConverterUtil.cpp:185`, then overwrites it from metadata only when `metadata != nullptr` (`ConverterUtil.cpp:186`). When metadata is non-null, the `scale` key is expected to always be present — if it were absent, `ArrowMetadataGetValue` would return an error and the conversion macro would stop and surface an explicit conversion failure to the caller; `std::stoi` is never invoked with a null pointer in this path. Snowflake's protocol guarantees the key is always included when metadata is present. Nanosecond scale (9) is common in practice but is not guaranteed by the wire format.

**Storage:** Values are stored as a signed 64-bit integer representing units since epoch, where one unit equals `10^(-scale)` seconds (e.g. nanoseconds when scale=9, microseconds when scale=6). There is no UTC offset attached and no UTC conversion — the wall-clock value is always preserved exactly.

---

## Python Return Type

**Type:** `datetime` with `tzinfo=None` (naive datetime).

**Evidence:**

- `python/src/snowflake/connector/_internal/arrow_context.py:108-109` (standard/Linux path):
  ```python
  def TIMESTAMP_NTZ_to_python(self, epoch: int, microseconds: int) -> datetime:
      return datetime.fromtimestamp(epoch, timezone.utc).replace(tzinfo=None) + timedelta(microseconds=microseconds)
  ```
  The `.replace(tzinfo=None)` call explicitly strips timezone info, making the returned datetime naive.

- `python/src/snowflake/connector/_internal/arrow_context.py:111-112` (Windows path):
  ```python
  def TIMESTAMP_NTZ_to_python_windows(self, epoch: int, microseconds: int) -> datetime:
      return ZERO_EPOCH + timedelta(seconds=epoch, microseconds=microseconds)
  ```
  The selection between the two paths is compile-time. Both `OneFieldTimeStampNTZConverter::toPyObject` (`TimeStampConverter.cpp:70-76`) and `TwoFieldTimeStampNTZConverter::toPyObject` (`TimeStampConverter.cpp:126-132`) independently use `#ifdef _WIN32` to call `TIMESTAMP_NTZ_to_python_windows` on Windows and `TIMESTAMP_NTZ_to_python` on all other platforms. (`ConverterUtil.cpp` selects between the One-field `INT64` and Two-field `STRUCT` Arrow wire formats based on the Arrow schema type.) Both paths produce identical results for all test scenario values (TS_2024_JAN, TS_EPOCH, TS_WITH_MICROSECONDS — all non-negative epochs). This follows by logical equivalence: `datetime.fromtimestamp(epoch, timezone.utc).replace(tzinfo=None)` equals `ZERO_EPOCH + timedelta(seconds=epoch)`, so both expressions compute the same value. The e2e tests run the standard path (Linux CI). The Windows path uses `ZERO_EPOCH + timedelta(seconds=epoch, microseconds=microseconds)`, where `ZERO_EPOCH = datetime.fromtimestamp(0, timezone.utc).replace(tzinfo=None)` equals `datetime(1970, 1, 1, 0, 0, 0)`. The standard path uses `datetime.fromtimestamp(epoch, timezone.utc).replace(tzinfo=None) + timedelta(microseconds=microseconds)` — it does not reference `ZERO_EPOCH` directly, but produces the same result because `datetime.fromtimestamp(epoch, timezone.utc).replace(tzinfo=None)` is equivalent to `ZERO_EPOCH + timedelta(seconds=epoch)` for all non-negative epochs. Equivalence for negative epochs (pre-1970) is **not in scope for this test suite** — see "Out of Scope" below.

- `python/tests/unit/test_arrow_context.py:171-178`: unit test asserts `result.tzinfo is None` for NTZ conversion.

- Old driver `/home/fpawlowski/repo/snowflake-connector-python/src/snowflake/connector/arrow_context.py:98`: identical `.replace(tzinfo=None)` pattern — confirms reference driver returns naive datetime.

---

## Binding Behaviour

**Python `datetime` instances send a `TIMESTAMP_NTZ` type tag in the bind protocol** (source: `python/src/snowflake/connector/_internal/type_codes.py:80`). Note: `struct_time` also maps to `TIMESTAMP_NTZ` in the same lookup table (`type_codes.py:84`) but takes a different code path that uses `time.mktime()` (local-time-dependent); `struct_time` binding is not in scope for these tests.
```python
"datetime": "TIMESTAMP_NTZ",
```

**Naive datetime (tzinfo=None):** stored as-is — the wall-clock value is preserved exactly. What you insert is what you get back.

**Tz-aware datetime:** converted to UTC, then tzinfo is stripped before binding (source: `python/src/snowflake/connector/_internal/binding_converters.py:128-130`):
```python
if dt.tzinfo is not None:
    dt = dt.astimezone(timezone.utc).replace(tzinfo=None)
```
Implication: binding `datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone(timedelta(hours=2)))` stores `2024-01-15 08:30:00` (the UTC wall-clock value), not the original local time. The result returned on `SELECT ?::TIMESTAMP_NTZ` will be `datetime(2024, 1, 15, 8, 30, 0)` — naive, UTC equivalent. Note: when the SQL expression is `?::TIMESTAMP_LTZ` instead, Snowflake receives the same NTZ-tagged value but interprets the bare wall-clock as session-local time, producing a different result.

**Encoding precision:** `epoch_seconds` is computed as `(dt - _ZERO_EPOCH).total_seconds()` (`binding_converters.py:131`). `timedelta.total_seconds()` is a C builtin in CPython; empirically, for all 2024-era test values it returns the same float64 as `float(n) / 1e6` where `n = (days * 86400 + seconds) * 10**6 + microseconds` (verified: see `same=True` for all test cases). The float error is bounded by half a ULP of the result magnitude — the same bound as the direct float64 representation of `epoch_seconds` itself. The epoch value is then serialised as `f"{epoch_seconds:f}".replace(".", "") + "000"` (`binding_converters.py:132-133`). `f"{x:f}"` formats to 6 decimal places. For the test scenario values, the encoding is lossless — confirmed by the actual computed outputs:
- `TS_2024_JAN` (whole-second): `1705314600.0` → `"1705314600000000000"` ✓
- `TS_WITH_MICROSECONDS` (`.123456`): `1705314600.123456` → `"1705314600123456000"` ✓
The `.123456` microsecond value encodes correctly (encoded string `1705314600123456000` matches expected). This holds for _all_ microsecond values across the entire 2024–2025 range: epoch values from `1.704×10⁹` (2024-01-01) to `1.750×10⁹` (2025-06) all fall in the float64 exponent band [2^30, 2^31), giving a constant ULP of 2^(30-52) = 2^(-22) ≈ 2.384×10⁻⁷. Since this is less than half a microsecond (5×10⁻⁷), `f"{x:f}"` always rounds to the correct microsecond. The ULP will not change until epoch 2^31 = 2147483648 (year 2038). Verified by checking ULP at Jan 2024, Jul 2024, Dec 2024, and Jun 2025 — all equal 2.384e-7. Note: `1705314600.999999` as float64 has exact decimal value `1705314600.99999904...` but still encodes to `1705314600999999000` correctly because the error is sub-half-microsecond. For far-future timestamps where the large integer part of the float consumes more significant digits, precision may degrade; such values are out of scope for these tests.

**Session timezone independence:** Because there is no UTC conversion on retrieval, the value returned is always the stored wall-clock — independent of session `TIMEZONE`. This makes NTZ test assertions deterministic with no UTC normalization needed.

---

## Differences from TIMESTAMP_LTZ

| Property | TIMESTAMP_NTZ | TIMESTAMP_LTZ |
|---|---|---|
| Python `tzinfo` | `None` (naive) | set (aware, non-None) |
| `assert_datetime_type` | `require_tzinfo=False` | `require_tzinfo=True` |
| SQL literal | `'2024-01-15 10:30:00'::TIMESTAMP_NTZ` | `'2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ` |
| UTC normalization in tests | not needed — direct `==` | `to_utc()` helper required |
| Session timezone effect | none — wall-clock preserved | values shift per session TZ |
| Binding a tz-aware datetime | UTC wall-clock stored as NTZ | driver sends wall-clock as NTZ; `?::TIMESTAMP_LTZ` cast then treats that bare value as session-local time |
| Large result set compare | default equality | `compare_ts_utc` comparator needed |

---

## Scenario Plan

**`tests/definitions/shared/types/timestamp_ntz.feature`** — 14 scenarios. Scenarios 1–10 have counterparts in LTZ coverage. Scenario 11 is new — it documents the UTC-strip binding behaviour. Scenarios 12–13 document `TIMESTAMP_TYPE_MAPPING` alias behaviour. Scenario 14 documents the nanosecond precision round-trip that JDBC/ODBC must implement; Python is permanently excluded (`@python_not_needed`). The reference driver confirms identical NTZ retrieval behaviour for scenarios 1–10 (naive datetime, no tzinfo).

| # | Scenario | Rationale | Old driver ref |
|---|---|---|---|
| 1 | Type casting — NTZ → naive datetime | Verify return type is `datetime` with `tzinfo=None` | `arrow_context.py:108-109` |
| 2 | SELECT literal: basic (two values) | Core SELECT path — two different timestamps | `test_timestamp_ltz.py` basic case |
| 3 | SELECT literal: epoch | Verify epoch (zero timestamp) boundary | `test_timestamp_ltz.py` epoch case |
| 4 | SELECT literal: microseconds | Verify microsecond precision is preserved | `test_timestamp_ltz.py` microseconds case |
| 5 | SELECT with NULL | Verify NULL is returned as Python `None` | `test_timestamp_ltz.py` null case |
| 6 | Large result set from GENERATOR (50 000 rows) | Multi-chunk download correctness | `test_timestamp_ltz.py` large result set |
| 7 | Table: basic, epoch, microseconds, null (outline) | Full round-trip insert→select | `test_timestamp_ltz.py` table cases |
| 8 | Table: large result set | Multi-chunk download from table | `test_timestamp_ltz.py` table large |
| 9 | Parameter binding: SELECT (naive) | Binding naive datetime to ?::TIMESTAMP_NTZ; exact round-trip verified | `test_timestamp_ltz.py` binding select |
| 10 | Parameter binding: INSERT via executemany | Bulk insert using multirow binding | `test_timestamp_ltz.py` binding insert |
| 11 | Parameter binding: tz-aware datetime stores UTC equivalent | Bind aware datetime with +02:00 offset; assert UTC naive value returned | new — documents UTC-strip behaviour with an assertion |
| 12 | TIMESTAMP/DATETIME alias → naive when mapping=TIMESTAMP_NTZ (outline) | Alias semantics depend on `TIMESTAMP_TYPE_MAPPING`; NTZ mapping returns naive datetime | new |
| 13 | TIMESTAMP alias → aware when mapping=TIMESTAMP_LTZ | Confirms alias is NOT always NTZ; different mapping returns aware datetime | new |
| 14 | Nanosecond precision preserved (JDBC/ODBC only) | `@jdbc_e2e @odbc_e2e @python_not_needed` — JDBC/ODBC must return all 9 nanosecond digits; Python is permanently excluded (scale 6 cap) | new |

Scenarios 2–4 are expressed as a single `Scenario Outline` with three Examples rows.
Scenario 7 is expressed as a single `Scenario Outline` with four Examples rows (basic, epoch, microseconds, null).
Scenario 12 is expressed as a `Scenario Outline` with two Examples rows (TIMESTAMP, DATETIME).

**`tests/definitions/python/types/timestamp_ntz.feature`** — 1 Python-specific scenario.

| # | Scenario | Rationale |
|---|---|---|
| P1 | Nanosecond precision truncated to microseconds | Python `datetime` is structurally capped at 6 decimal places; digits 7–9 from Snowflake scale=9 data are silently dropped. Permanent Python exclusion — not a TODO for other drivers. |

This scenario lives in `python/` because the truncation is a Python type-system limitation: JDBC/ODBC will preserve nanosecond precision and must not inherit this scenario as a TODO. The canonical nanosecond round-trip scenario (Scenario 14 in `shared/`, tagged `@jdbc_e2e @odbc_e2e @python_not_needed`) documents the desired state for non-Python drivers; Python is explicitly and permanently excluded.

---

## Edge Cases

- **Epoch (zero timestamp):** `datetime(1970, 1, 1, 0, 0, 0)` — tests that the epoch boundary is handled correctly without off-by-one errors in the epoch arithmetic.
- **Microsecond precision:** `datetime(2024, 1, 15, 10, 30, 0, 123456)` — confirms 6-decimal-place precision is preserved end-to-end.
- **Nanosecond truncation:** input `'2024-01-15 10:30:00.123456789'` (scale=9) — Python returns `datetime(2024, 1, 15, 10, 30, 0, 123456)`, digits 7–9 dropped silently.
- **NULL:** confirms `None` is returned (not a sentinel datetime value).
- **Large result set (50 000 rows):** triggers multi-chunk Arrow download; verifies sequential correctness across chunk boundaries.
- **Binding tz-aware datetime:** the UTC-strip behaviour is verified with an e2e assertion. Binding `datetime(2024, 1, 15, 12, 30, 0, tzinfo=timezone(timedelta(hours=2)))` (UTC+2) must return `datetime(2024, 1, 15, 10, 30, 0)` — the naive UTC equivalent. The round-trip is fully deterministic given a fixed offset.

---

## Out of Scope

- **Nanosecond full-precision round-trip (JDBC/ODBC):** The Gherkin scenario is already present in `shared/` (Scenario 14, `@jdbc_e2e @odbc_e2e @python_not_needed`). JDBC/ODBC test implementations are deferred to when those drivers implement TIMESTAMP_NTZ. Python is permanently excluded — see Scenario P1 and `python/types/timestamp_ntz.feature` for the Python-specific truncation test.
- **Numpy variant:** `numpy.datetime64` return path is not in scope for these e2e tests.
- **Min/max date boundaries:** Not tested — boundary values can vary by Snowflake account precision settings and are not part of standard connector contract.
- **Pre-1970 (negative epoch) timestamps:** The standard and Windows paths are believed to produce equivalent results for negative epochs, but this is not verified. Testing pre-1970 values is not in scope — they are not part of the standard type-coverage contract and require additional investigation of platform-specific `datetime.fromtimestamp` behaviour.
- **Session timezone effect:** Not tested as a negative. This is an explicit scope decision — varying the session timezone to confirm NTZ invariance is beyond the stated test contract (see "Session timezone independence" in the main section for driver-level evidence). Server-side NTZ semantics are a Snowflake platform guarantee; testing them would require a dedicated session-TZ test fixture outside the scope of this type-coverage suite.

---

## Implementation Status

- [x] Design document written (this file)
- [x] Shared Gherkin feature file: `tests/definitions/shared/types/timestamp_ntz.feature` (14 scenarios)
- [x] Python-specific Gherkin feature file: `tests/definitions/python/types/timestamp_ntz.feature` (1 scenario — nanosecond truncation)
- [x] Python tests: `python/tests/e2e/types/test_timestamp_ntz.py`
