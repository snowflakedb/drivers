"""
End-to-end coverage for the `use_s3_regional_url` kwarg and its
deprecated alias `enable_stage_s3_privatelink_for_us_east_1`.

We can't assert that the regional S3 endpoint is actually used — that
would require a PrivateLink-enabled test account — but we can verify
that:
  * the kwarg is accepted by the public `connect()` API and a basic PUT
    round-trips successfully (run against both drivers — the universal
    driver receives the canonical name, the reference connector receives
    the legacy name, since each is the only spelling that driver accepts);
  * the legacy kwarg emits a `DeprecationWarning` at connection time
    (universal driver only — the reference connector exposes it as a
    non-deprecated kwarg).

The OR-with-stage-info-flags logic is covered by Rust unit tests in
`sf_core/src/rest/snowflake/query_response.rs::tests`, and the
kwarg-rewrite logic by `tests/unit/test_connection_config.py`.
"""

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.e2e.put_get.put_get_helper import (
    create_temporary_stage_and_upload_file,
)
from tests.utils import shared_test_data_dir


# The reference connector only accepts `enable_stage_s3_privatelink_for_us_east_1`;
# the universal driver accepts both but `use_s3_regional_url` is canonical and the
# legacy name emits a DeprecationWarning. Pick the spelling each driver expects so
# the round-trip test exercises real reference coverage of the underlying feature.
_REGIONAL_URL_KWARG = "use_s3_regional_url" if IS_UNIVERSAL_DRIVER else "enable_stage_s3_privatelink_for_us_east_1"


def test_s3_regional_url_kwarg_accepts_and_round_trips(connection_factory):
    # The kwarg is harmless on non-S3 stages (the OR only affects the S3
    # path), so this test runs on any cloud-backed test account. The
    # point is to prove the kwarg flows from the public `connect()` API
    # through the wrapper (and, for universal, the Rust core and
    # file-transfer agent).
    file_path = shared_test_data_dir() / "overwrite" / "original" / "test_data.csv"

    # Given a connection opened with the regional-URL kwarg set
    with connection_factory(**{_REGIONAL_URL_KWARG: True}) as conn:
        with conn.cursor() as cursor:
            # When a file is uploaded to a temporary stage
            stage_name, upload_result = create_temporary_stage_and_upload_file(
                cursor,
                "TEST_S3_REGIONAL_URL_KWARG",
                file_path,
                auto_compress=False,
                overwrite=True,
            )

            # Then the upload succeeds and the round-trip read matches
            assert upload_result[6] == "UPLOADED"
            cursor.execute(f"SELECT $1, $2, $3 FROM @{stage_name}")
            row = cursor.fetchone()
            assert row == ("original", "test", "data")


@pytest.mark.skip_reference(
    reason="Reference Python connector exposes enable_stage_s3_privatelink_for_us_east_1 "
    "as a non-deprecated kwarg; the universal driver demotes it to a deprecated alias."
)
def test_legacy_kwarg_emits_deprecation_warning_on_connection(connection_factory):
    # The deprecation rewrite happens inside `from_kwargs`. Catching it
    # on the connection-construction path proves it is wired through
    # the public `connect()` API and not just the unit-tested config
    # layer.

    # When opening a connection with the legacy kwarg
    # Then a DeprecationWarning naming the legacy kwarg is emitted
    with pytest.warns(DeprecationWarning, match="enable_stage_s3_privatelink_for_us_east_1"):
        with connection_factory(enable_stage_s3_privatelink_for_us_east_1=True) as conn:
            assert conn is not None
