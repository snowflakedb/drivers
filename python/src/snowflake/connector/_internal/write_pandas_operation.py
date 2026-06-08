"""Write pandas DataFrame to Snowflake table via Parquet stage upload pipeline.

This module contains WritePandasConfig (validated parameters) and
WritePandasOperation (pipeline execution).  The public API lives in
snowflake.connector.pandas_tools.
"""

from __future__ import annotations

import secrets
import warnings

from collections.abc import Callable, Iterator
from logging import getLogger
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TYPE_CHECKING, Any, Literal, NamedTuple, cast

from ..cursor import SnowflakeCursor
from ..errors import ProgrammingError
from .extras import pandas


if TYPE_CHECKING:
    from pandas import DataFrame

    from ..connection import Connection

logger = getLogger(__name__)

# Maps user-facing compression name → Snowflake FILE_FORMAT COMPRESSION value.
# Parquet FILE_FORMAT doesn't accept "gzip" directly; "auto" handles it.
# https://docs.snowflake.com/en/sql-reference/sql/copy-into-table#type-parquet
VALID_COMPRESSIONS_MAP: dict[str, str] = {"gzip": "auto", "snappy": "snappy"}

VALID_TABLE_TYPES: set[str] = {"", "temp", "temporary", "transient"}

ALLOWED_ICEBERG_CONFIGS: set[str] = {
    "EXTERNAL_VOLUME",
    "CATALOG",
    "BASE_LOCATION",
    "CATALOG_SYNC",
    "STORAGE_SERIALIZATION_POLICY",
}


# ---------------------------------------------------------------------------
# Module-level utilities (stateless, pure functions)
# ---------------------------------------------------------------------------


def quote_identifier(name: str) -> str:
    """Double-quote a SQL identifier, escaping internal double quotes."""
    return '"' + name.replace('"', '""') + '"'


def qualify_name(
    database: str | None,
    schema: str | None,
    name: str,
    quote_identifiers: bool,
) -> str:
    """Build a fully-qualified object name, optionally quoting each part."""
    parts = [p for p in (database, schema, name) if p is not None]
    if quote_identifiers:
        parts = [quote_identifier(p) for p in parts]
    return ".".join(parts)


def escape_path_for_sql(path: str) -> str:
    """Escape backslashes and single quotes for use inside SQL string literals."""
    return path.replace("\\", "\\\\").replace("'", "\\'")


def generate_temp_name(prefix: str) -> str:
    """Generate a random temporary object name."""
    return f"__WRITE_PANDAS_{prefix}_{secrets.token_hex(8)}"


def _sql_bool(value: bool) -> str:
    """Convert Python bool to SQL TRUE/FALSE literal."""
    return "TRUE" if value else "FALSE"


def _convert_value_to_sql_option(value: str | bool | int | float) -> str:
    """Convert a Python value to a SQL option literal for iceberg config."""
    if isinstance(value, str):
        if len(value) > 1 and value.startswith("'") and value.endswith("'"):
            return value
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    return str(value)


def _create_temp_object(
    cursor: SnowflakeCursor,
    sql_builder: Callable[[str], str],
    qualified_name: str,
    bare_name: str,
) -> str:
    """Try creating in target schema; fall back to current schema on privilege error."""
    try:
        cursor.execute(sql_builder(qualified_name))
        return qualified_name
    except ProgrammingError:
        cursor.execute(sql_builder(bare_name))
        return bare_name


def _drop_object(cursor: SnowflakeCursor, name: str, object_type: str) -> None:
    """Drop a Snowflake object if it exists."""
    cursor.execute(f"DROP {object_type} IF EXISTS {name}")


# ---------------------------------------------------------------------------
# WritePandasResult
# ---------------------------------------------------------------------------


class WritePandasResult(NamedTuple):
    """Result of a write_pandas invocation. Backward-compatible with plain tuple."""

    success: bool
    nchunks: int
    nrows: int
    copy_results: list


# ---------------------------------------------------------------------------
# WritePandasConfig
# ---------------------------------------------------------------------------


class WritePandasConfig:
    """Validated, immutable configuration for a write_pandas invocation.

    All input parameters are normalized and validated at construction time.
    Raises ProgrammingError or ValueError for invalid inputs.
    """

    def __init__(
        self,
        conn: Connection,
        df: DataFrame,
        table_name: str,
        *,
        database: str | None = None,
        schema: str | None = None,
        chunk_size: int | None = None,
        compression: Literal["gzip", "snappy"] = "gzip",
        on_error: str = "abort_statement",
        parallel: int = 4,
        quote_identifiers: bool = True,
        infer_schema: bool = False,
        auto_create_table: bool = False,
        overwrite: bool = False,
        table_type: Literal["", "temp", "temporary", "transient"] = "",
        use_logical_type: bool | None = None,
        iceberg_config: dict[str, str] | None = None,
        bulk_upload_chunks: bool = False,
        use_vectorized_scanner: bool = False,
        **parquet_kwargs: Any,
    ) -> None:
        self.conn = conn
        self.df = df
        self.table_name = table_name
        self.database = database
        self.schema = schema
        self.chunk_size = chunk_size if chunk_size is not None else len(df)
        self.compression = compression
        self.on_error = on_error
        self.parallel = parallel
        self.quote_identifiers = quote_identifiers
        self.infer_schema = infer_schema
        self.auto_create_table = auto_create_table
        self.overwrite = overwrite
        self.table_type = table_type
        self.use_logical_type = use_logical_type
        self.iceberg_config = iceberg_config
        self.bulk_upload = bulk_upload_chunks
        self.use_vectorized_scanner = use_vectorized_scanner
        self.parquet_kwargs = parquet_kwargs

        self._validate()

    # -- Validation ---------------------------------------------------------

    def _validate(self) -> None:
        if self.database is not None and self.schema is None:
            raise ProgrammingError(msg="schema must be provided when database is specified")
        if self.compression not in VALID_COMPRESSIONS_MAP:
            raise ProgrammingError(
                msg=f"Invalid compression: '{self.compression}'. "
                f"Supported values: {', '.join(sorted(VALID_COMPRESSIONS_MAP))}"
            )
        if self.table_type.lower() not in VALID_TABLE_TYPES:
            raise ValueError(
                f"Invalid table_type: '{self.table_type}'. "
                f"Supported values: "
                f"{', '.join(repr(t) for t in sorted(VALID_TABLE_TYPES))}"
            )
        if self.iceberg_config:
            normalized = {k.upper() for k in self.iceberg_config}
            invalid = normalized - ALLOWED_ICEBERG_CONFIGS
            if invalid:
                raise ProgrammingError(
                    msg=f"Invalid iceberg configurations option(s) provided {', '.join(sorted(invalid))}"
                )

    # -- Branching properties -----------------------------------------------

    @property
    def needs_inference(self) -> bool:
        return self.auto_create_table or self.overwrite or self.infer_schema

    @property
    def needs_table_creation(self) -> bool:
        return self.auto_create_table or self.overwrite

    @property
    def needs_truncate(self) -> bool:
        return self.overwrite and not self.auto_create_table

    @property
    def needs_swap(self) -> bool:
        return self.overwrite and self.auto_create_table

    @property
    def binary_as_text_false_on_stage(self) -> bool:
        return self.auto_create_table or self.overwrite

    @property
    def binary_as_text_false_on_copy(self) -> bool:
        return self.auto_create_table or self.overwrite or self.infer_schema

    # -- DataFrame inspection -----------------------------------------------

    def has_tz_aware_columns(self) -> bool:
        return any(hasattr(dtype, "tz") and dtype.tz is not None for dtype in self.df.dtypes)

    def is_standard_range_index(self) -> bool:
        idx = self.df.index
        return isinstance(idx, pandas.RangeIndex) and idx.start == 0 and idx.step == 1

    # -- Helpers ------------------------------------------------------------

    def qualify(self, name: str) -> str:
        """Build a fully-qualified name using this config's database/schema."""
        return qualify_name(self.database, self.schema, name, self.quote_identifiers)

    def emit_warnings(self) -> None:
        """Emit user-facing warnings about the input data.

        Called from execute() rather than __init__ so that stacklevel
        correctly points to the caller of write_pandas().
        """
        if self.use_logical_type is not True and self.has_tz_aware_columns():
            warnings.warn(
                "DataFrame contains a datetime column with timezone "
                "information. This data may not round-trip correctly unless "
                "use_logical_type=True is set.",
                UserWarning,
                stacklevel=4,
            )
        if not self.is_standard_range_index():
            warnings.warn(
                "The DataFrame has a non-standard index which will not be "
                "written. Consider resetting the index with "
                "df.reset_index().",
                UserWarning,
                stacklevel=4,
            )


# ---------------------------------------------------------------------------
# WritePandasOperation
# ---------------------------------------------------------------------------


class WritePandasOperation:
    """Executes the write_pandas pipeline from a validated config.

    Accepts a single WritePandasConfig.  execute() runs the full pipeline
    without mutating self — all intermediate state is threaded through
    local variables and return values.
    """

    def __init__(self, cfg: WritePandasConfig) -> None:
        self._cfg = cfg

    # -- Main pipeline ---------------------------------------------------

    def execute(self) -> WritePandasResult:
        """Run the full pipeline. Returns a WritePandasResult (named tuple)."""
        cfg = self._cfg
        cfg.emit_warnings()

        cursor = cast(SnowflakeCursor, cfg.conn.cursor(SnowflakeCursor))
        target_location: str | None = None
        try:
            stage_location = self._create_stage(cursor)
            nchunks, nrows = self._upload_to_stage(cursor, stage_location)

            column_type_map: dict[str, str] | None = None
            if cfg.needs_inference:
                file_format_location = self._create_file_format(cursor)
                column_type_map = self._infer_column_types(cursor, stage_location, file_format_location)

            target_location = self._resolve_target_table()

            if cfg.needs_table_creation:
                self._create_table(cursor, target_location, column_type_map)

            if cfg.needs_truncate:
                self._truncate_table(cursor, target_location)

            copy_results = self._copy_into(cursor, stage_location, target_location, column_type_map)

            if cfg.needs_swap:
                self._swap_tables(cursor, target_location)

        except Exception:
            if cfg.needs_swap and target_location:
                try:
                    _drop_object(cursor, target_location, "TABLE")
                except Exception:
                    logger.warning(
                        "write_pandas failed and could not drop temporary "
                        "staging table %s created for overwrite swap — "
                        "it may need to be dropped manually",
                        target_location,
                        exc_info=True,
                    )
            raise
        finally:
            cursor.close()

        success = all(row[1] == "LOADED" for row in copy_results)
        return WritePandasResult(success, nchunks, nrows, copy_results)

    # -- Stage & upload --------------------------------------------------

    def _create_stage(self, cursor: SnowflakeCursor) -> str:
        cfg = self._cfg
        name = generate_temp_name("STAGE")
        qualified = cfg.qualify(name)
        return _create_temp_object(cursor, self._build_create_stage_sql, qualified, name)

    def _upload_to_stage(self, cursor: SnowflakeCursor, stage_location: str) -> tuple[int, int]:
        """Write DataFrame chunks to Parquet and PUT to stage.

        Returns (nchunks, nrows).
        """
        cfg = self._cfg
        nchunks = 0
        nrows = 0
        with TemporaryDirectory() as tmp_dir:
            for idx, chunk in self._iter_chunks():
                chunk_path = Path(tmp_dir) / f"file{idx}.txt"
                chunk.to_parquet(path=chunk_path, compression=cfg.compression, **cfg.parquet_kwargs)
                nchunks += 1
                nrows += len(chunk)

                if not cfg.bulk_upload:
                    self._put_file(cursor, stage_location, chunk_path)
                    chunk_path.unlink()

            if cfg.bulk_upload:
                self._put_directory(cursor, stage_location, tmp_dir)

        return nchunks, nrows

    def _put_file(self, cursor: SnowflakeCursor, stage_location: str, path: Path) -> None:
        uri = escape_path_for_sql(path.as_posix())
        cursor.execute(
            f"PUT 'file://{uri}' @{stage_location} "
            f"PARALLEL={self._cfg.parallel} AUTO_COMPRESS=FALSE "
            f"SOURCE_COMPRESSION=AUTO_DETECT OVERWRITE=TRUE"
        )

    def _put_directory(self, cursor: SnowflakeCursor, stage_location: str, directory: str) -> None:
        uri = escape_path_for_sql(Path(directory).as_posix())
        cursor.execute(
            f"PUT 'file://{uri}/*' @{stage_location} "
            f"PARALLEL={self._cfg.parallel} AUTO_COMPRESS=FALSE "
            f"SOURCE_COMPRESSION=AUTO_DETECT OVERWRITE=TRUE"
        )

    def _iter_chunks(self) -> Iterator[tuple[int, DataFrame]]:
        """Yield (index, chunk_df). Empty DataFrame yields (0, empty_df)."""
        cfg = self._cfg
        if len(cfg.df) == 0 or cfg.chunk_size == 0:
            yield 0, cfg.df
            return
        for idx, start in enumerate(range(0, len(cfg.df), cfg.chunk_size)):
            yield idx, cfg.df.iloc[start : start + cfg.chunk_size]

    # -- Schema inference ------------------------------------------------

    def _create_file_format(self, cursor: SnowflakeCursor) -> str:
        cfg = self._cfg
        name = generate_temp_name("FILE_FORMAT")
        qualified = cfg.qualify(name)
        return _create_temp_object(cursor, self._build_create_file_format_sql, qualified, name)

    def _infer_column_types(
        self,
        cursor: SnowflakeCursor,
        stage_location: str,
        file_format_location: str,
    ) -> dict[str, str]:
        """Run INFER_SCHEMA and return {UPPER_COL_NAME: SQL_TYPE} mapping."""
        escaped_stage = stage_location.replace("'", "\\'")
        rows = cursor.execute(
            f"SELECT * FROM TABLE(INFER_SCHEMA("
            f"LOCATION => '@{escaped_stage}', "
            f"FILE_FORMAT => '{file_format_location}'))"
        ).fetchall()
        return {row[0].upper(): row[1] for row in rows}

    # -- Table management ------------------------------------------------

    def _resolve_target_table(self) -> str:
        cfg = self._cfg
        if cfg.needs_swap:
            return cfg.qualify(generate_temp_name("TABLE"))
        return cfg.qualify(cfg.table_name)

    def _create_table(
        self,
        cursor: SnowflakeCursor,
        target_location: str,
        column_type_map: dict[str, str] | None,
    ) -> None:
        cfg = self._cfg
        col_defs = []
        for col in cfg.df.columns:
            col_type = column_type_map.get(col.upper(), "VARIANT") if column_type_map else "VARIANT"
            col_name = quote_identifier(col) if cfg.quote_identifiers else col
            col_defs.append(f"{col_name} {col_type}")

        table_type_clause = cfg.table_type.upper() + " " if cfg.table_type else ""
        iceberg_prefix = "ICEBERG " if cfg.iceberg_config else ""
        iceberg_clause = self._build_iceberg_config_sql() if cfg.iceberg_config else ""

        cursor.execute(
            f"CREATE {table_type_clause}{iceberg_prefix}TABLE IF NOT EXISTS "
            f"{target_location} ({', '.join(col_defs)}) {iceberg_clause}"
        )

    def _truncate_table(self, cursor: SnowflakeCursor, target_location: str) -> None:
        cursor.execute(f"TRUNCATE TABLE IF EXISTS {target_location}")

    def _swap_tables(self, cursor: SnowflakeCursor, target_location: str) -> None:
        """Replace original table with temp target via DROP + RENAME.

        ALTER TABLE SWAP WITH would be atomic but doesn't work when the
        original table doesn't exist yet or tables are different types
        (TEMPORARY vs permanent).
        """
        cfg = self._cfg
        original = cfg.qualify(cfg.table_name)
        _drop_object(cursor, original, "TABLE")
        cursor.execute(f"ALTER TABLE {target_location} RENAME TO {original}")

    # -- COPY INTO -------------------------------------------------------

    def _copy_into(
        self,
        cursor: SnowflakeCursor,
        stage_location: str,
        target_location: str,
        column_type_map: dict[str, str] | None,
    ) -> list:
        sql = self._build_copy_into_sql(stage_location, target_location, column_type_map)
        return cursor.execute(sql).fetchall()

    def _build_copy_into_sql(
        self,
        stage_location: str,
        target_location: str,
        column_type_map: dict[str, str] | None,
    ) -> str:
        cfg = self._cfg
        target_cols: list[str] = []
        select_exprs: list[str] = []

        for col in cfg.df.columns:
            col_name = quote_identifier(col) if cfg.quote_identifiers else col
            target_cols.append(col_name)

            parquet_ref = f'$1:"{col}"'
            if column_type_map and col.upper() in column_type_map:
                parquet_ref += f"::{column_type_map[col.upper()]}"
            select_exprs.append(f"{parquet_ref} AS {col_name}")

        escaped_stage = stage_location.replace("'", "\\'")
        compression_mapped = VALID_COMPRESSIONS_MAP[cfg.compression]

        file_format_parts = [f"TYPE=PARQUET COMPRESSION={compression_mapped}"]
        if cfg.binary_as_text_false_on_copy:
            file_format_parts.append("BINARY_AS_TEXT=FALSE")
        if cfg.use_logical_type is not None:
            file_format_parts.append(f"USE_LOGICAL_TYPE={_sql_bool(cfg.use_logical_type)}")
        if cfg.use_vectorized_scanner:
            file_format_parts.append(f"USE_VECTORIZED_SCANNER={_sql_bool(cfg.use_vectorized_scanner)}")

        return (
            f"COPY INTO {target_location} ({', '.join(target_cols)}) "
            f"FROM (SELECT {', '.join(select_exprs)} "
            f"FROM '@{escaped_stage}') "
            f"FILE_FORMAT = ({' '.join(file_format_parts)}) "
            f"PURGE=TRUE ON_ERROR={cfg.on_error}"
        )

    # -- Shared helpers --------------------------------------------------

    def _build_create_stage_sql(self, name: str) -> str:
        cfg = self._cfg
        mapped = VALID_COMPRESSIONS_MAP[cfg.compression]
        fmt_opts = [f"TYPE=PARQUET COMPRESSION={mapped}"]
        if cfg.binary_as_text_false_on_stage:
            fmt_opts.append("BINARY_AS_TEXT=FALSE")
        return f"CREATE TEMPORARY STAGE {name} FILE_FORMAT=({' '.join(fmt_opts)})"

    def _build_create_file_format_sql(self, name: str) -> str:
        cfg = self._cfg
        mapped = VALID_COMPRESSIONS_MAP[cfg.compression]
        parts = [f"CREATE TEMPORARY FILE FORMAT {name} TYPE=PARQUET COMPRESSION={mapped}"]
        if cfg.use_logical_type is not None:
            parts.append(f"USE_LOGICAL_TYPE={_sql_bool(cfg.use_logical_type)}")
        return " ".join(parts)

    def _build_iceberg_config_sql(self) -> str:
        cfg = self._cfg
        if not cfg.iceberg_config:
            return ""
        normalized = {
            k.upper(): _convert_value_to_sql_option(v) for k, v in cfg.iceberg_config.items() if v is not None
        }
        return " ".join(f"{k}={v}" for k, v in normalized.items())
