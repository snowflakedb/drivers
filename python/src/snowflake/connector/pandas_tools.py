"""Public API for writing pandas DataFrames to Snowflake tables.

Usage:
    from snowflake.connector.pandas_tools import write_pandas
    success, nchunks, nrows, _ = write_pandas(conn, df, 'my_table')

Implementation details are in snowflake.connector._internal.write_pandas_operation.
"""

from __future__ import annotations

from collections.abc import Iterable
from functools import partial, wraps
from typing import TYPE_CHECKING, Any, Callable, Literal, TypeVar, cast

from ._internal.errorhandler import route_exception
from ._internal.extras import pandas, requires_dependency, sqlalchemy
from ._internal.write_pandas_operation import (
    WritePandasConfig,
    WritePandasOperation,
    WritePandasResult,
)
from .errors import Error, ProgrammingError


if TYPE_CHECKING:
    from pandas import DataFrame
    from pandas.io.sql import SQLTable
    from sqlalchemy import engine

    from .connection import Connection

F = TypeVar("F", bound=Callable[..., Any])


def _reject_kwargs(*names: str) -> Callable[[F], F]:
    """Reject the listed *names* if they appear in ``**kwargs``.

    Both ``pd_writer`` and ``make_pd_writer`` accept open-ended
    ``**kwargs`` that are forwarded to :func:`write_pandas`.  Some of
    those keyword names (``table``, ``conn``, ``keys``, ``data_iter``,
    ``df``, ``table_name``, ``schema``) overlap with arguments that
    pandas' ``to_sql`` injects automatically or that ``pd_writer``
    derives from SQLAlchemy objects.

    If a caller accidentally passes one of them, ``functools.partial``
    (in ``make_pd_writer``) or ``pd_writer`` itself would silently
    pre-bind the value and then collide with the positional argument
    that pandas supplies at call time, producing a confusing
    ``TypeError: got multiple values for argument '...'``.

    This decorator turns that into an explicit
    :class:`~snowflake.connector.errors.ProgrammingError` with a
    clear message *before* the call is made.
    """
    forbidden = frozenset(names)

    def decorator(func: F) -> F:
        @wraps(func)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            found = forbidden & kwargs.keys()
            if found:
                raise ProgrammingError(
                    f"{', '.join(sorted(found))} cannot be passed to {func.__name__}; "
                    f"{'it is' if len(found) == 1 else 'they are'} derived automatically."
                )
            return func(*args, **kwargs)

        return cast(F, wrapper)

    return decorator


@requires_dependency(pandas)
def write_pandas(
    conn: Connection,
    df: DataFrame,
    table_name: str,
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
    **kwargs: Any,
) -> WritePandasResult:
    """Write a pandas DataFrame to a Snowflake table via Parquet stage upload.

    Returns a WritePandasResult named tuple (success, nchunks, nrows, copy_results).
    Backward-compatible with plain tuple unpacking and indexing.
    """
    try:
        cfg = WritePandasConfig(
            conn,
            df,
            table_name,
            database=database,
            schema=schema,
            chunk_size=chunk_size,
            compression=compression,
            on_error=on_error,
            parallel=parallel,
            quote_identifiers=quote_identifiers,
            infer_schema=infer_schema,
            auto_create_table=auto_create_table,
            overwrite=overwrite,
            table_type=table_type,
            use_logical_type=use_logical_type,
            iceberg_config=iceberg_config,
            bulk_upload_chunks=bulk_upload_chunks,
            use_vectorized_scanner=use_vectorized_scanner,
            **kwargs,
        )
        return WritePandasOperation(cfg).execute()
    except Error as exc:
        # TODO: consider a function-level @with_errorhandler decorator
        #  if more free functions need this pattern in the future.
        route_exception(conn, None, exc)


@requires_dependency(pandas, sqlalchemy)
@_reject_kwargs("table", "conn", "keys", "data_iter")
def make_pd_writer(
    **kwargs: Any,
) -> Callable:
    """Return a ``pd_writer`` with pre-bound keyword arguments.

    Useful when you need to pass extra options (e.g. ``parallel``,
    ``quote_identifiers``) through ``DataFrame.to_sql(method=...)``.

    Example::

        df.to_sql("t", engine, index=False, method=make_pd_writer(parallel=1, quote_identifiers=False))
    """
    return partial(pd_writer, **kwargs)


@requires_dependency(pandas, sqlalchemy)
@_reject_kwargs("conn", "df", "table_name", "schema")
def pd_writer(
    table: SQLTable,
    conn: engine.Engine | engine.Connection,
    keys: Iterable,
    data_iter: Iterable,
    **kwargs: Any,
) -> None:
    """Adapter for ``DataFrame.to_sql(method=pd_writer)``.

    Wraps :func:`write_pandas` so it satisfies the pandas ``method`` callable
    protocol.  Extra ``**kwargs`` are forwarded to ``write_pandas`` (except
    ``conn``, ``df``, ``table_name``, and ``schema`` which are derived from
    the SQLAlchemy objects).

    Note:
        Lower-case column names must be quoted in the DataFrame (e.g.
        ``'"my_col"'``) because snowflake-sqlalchemy creates tables
        case-insensitively while ``write_pandas`` quotes columns by default.

    Example::

        df.to_sql("driver_versions", engine, index=False, method=pd_writer)
    """
    sf_connection = conn.connection.connection
    df = pandas.DataFrame(data_iter, columns=keys)
    write_pandas(
        conn=sf_connection,
        df=df,
        table_name=table.name.upper(),
        schema=table.schema,
        **kwargs,
    )
