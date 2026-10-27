"""GET of files that the server wrote to a stage with COPY INTO.

Snowpark's ``DataFrame.to_polars()`` unloads the result as Snappy Parquet with
``COPY INTO @stage``, downloads ``data_0_0_0.snappy.parquet`` with GET, and
reads it with ``polars.read_parquet``. The tests replay that flow and check the
downloaded bytes directly, so a GET that writes anything other than the stored
Parquet file fails here rather than as a missing ``PAR1`` footer in polars.
"""

import gzip
import tempfile
import uuid

from pathlib import Path

import pytest

from tests.e2e.put_get.put_get_helper import as_file_uri


PARQUET_MAGIC = b"PAR1"
GZIP_MAGIC = b"\x1f\x8b"

ENCRYPTIONS = [
    pytest.param(None, id="default-encryption"),
    pytest.param("SNOWFLAKE_SSE", id="snowflake-sse"),
]


def _create_stage(cursor, prefix: str, encryption: str | None) -> str:
    stage_name = f"{prefix}_{uuid.uuid4().hex}".upper()
    encryption_clause = f" ENCRYPTION = (TYPE = '{encryption}')" if encryption else ""
    cursor.execute(f"CREATE TEMPORARY STAGE IF NOT EXISTS {stage_name}{encryption_clause}")
    return stage_name


def _unload(cursor, stage_name: str, file_format: str) -> str:
    sub_path = f"unload/{uuid.uuid4().hex[:12]}/"
    cursor.execute(
        f"COPY INTO @{stage_name}/{sub_path} "
        "FROM (SELECT 42::INT AS i, 'hello'::VARCHAR AS s, TRUE AS b, DATE '2024-01-01' AS d) "
        f"FILE_FORMAT = ({file_format}) OVERWRITE = TRUE HEADER = TRUE"
    )
    cursor.execute(f"LIST @{stage_name}/{sub_path}")
    names = [row[0].rsplit("/", 1)[-1] for row in cursor.fetchall()]
    assert len(names) == 1, f"expected one unloaded file, got {names}"
    return f"{sub_path}{names[0]}"


def _get(cursor, stage_name: str, stage_path: str, download_dir: Path) -> bytes:
    cursor.execute(f"GET @{stage_name}/{stage_path} 'file://{as_file_uri(download_dir)}/'")
    row = cursor.fetchone()
    assert row is not None
    assert row[2] == "DOWNLOADED"
    local_file = download_dir / stage_path.rsplit("/", 1)[-1]
    assert local_file.exists(), f"GET did not write {local_file}"
    return local_file.read_bytes()


@pytest.mark.parametrize("encryption", ENCRYPTIONS)
def test_should_get_unloaded_snappy_parquet_as_a_valid_parquet_file(connection, encryption):
    with connection.cursor() as cursor:
        # Given Stage with an unloaded Snappy Parquet file, as written by Snowpark to_polars()
        stage_name = _create_stage(cursor, "TEST_GET_UNLOADED_PARQUET", encryption)
        stage_path = _unload(cursor, stage_name, "TYPE = PARQUET COMPRESSION = 'SNAPPY'")
        assert stage_path.endswith(".snappy.parquet")

        with tempfile.TemporaryDirectory() as temp_dir:
            # When File is downloaded using GET command
            content = _get(cursor, stage_name, stage_path, Path(temp_dir))

        # Then Downloaded file should start and end with the Parquet magic
        assert not content.startswith(GZIP_MAGIC), (
            f"GET wrote a gzip stream for {stage_path} ({len(content)} bytes, first bytes {content[:8].hex()})"
        )
        assert content[:4] == PARQUET_MAGIC, f"first bytes {content[:8].hex()}, {len(content)} bytes"
        assert content[-4:] == PARQUET_MAGIC, f"last bytes {content[-8:].hex()}, {len(content)} bytes"


@pytest.mark.parametrize("encryption", ENCRYPTIONS)
def test_should_get_unloaded_gzip_csv_as_stored(connection, encryption):
    with connection.cursor() as cursor:
        # Given Stage with an unloaded gzip CSV file
        stage_name = _create_stage(cursor, "TEST_GET_UNLOADED_GZIP_CSV", encryption)
        stage_path = _unload(cursor, stage_name, "TYPE = CSV COMPRESSION = GZIP")
        assert stage_path.endswith(".csv.gz")

        with tempfile.TemporaryDirectory() as temp_dir:
            # When File is downloaded using GET command
            content = _get(cursor, stage_name, stage_path, Path(temp_dir))

        # Then Downloaded file should still be gzip
        assert content[:2] == GZIP_MAGIC, f"first bytes {content[:8].hex()}, {len(content)} bytes"

        # And Decompress to the unloaded row
        lines = gzip.decompress(content).decode().splitlines()
        assert lines == ["I,S,B,D", "42,hello,true,2024-01-01"]
