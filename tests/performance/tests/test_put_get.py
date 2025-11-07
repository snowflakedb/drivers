import pytest
from runner.test_types import TestType

S3_TEST_DATA_12MX100 = "s3://sfc-eng-data/ecosystem/12Mx100/"


@pytest.mark.iterations(2)
@pytest.mark.warmup_iterations(0)
def test_put_files_12mx100(perf_test):
    """
    PUT test: Upload 100 files of 12MB each from local disk to temporary stage.
    """
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX100,
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE overwrite=true"
        )
    )


@pytest.mark.iterations(2)
@pytest.mark.warmup_iterations(0)
def test_get_files_12mx100(perf_test):
    """
    GET test: Download 100 files of 12MB each from temporary stage to local disk.
    """
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX100,
        
        setup_queries=[
            "CREATE TEMPORARY STAGE get_test_stage",
            "PUT file:///put_get_files/* @get_test_stage "
            "AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE overwrite=false"
        ],
        sql_command=(
            "GET @get_test_stage "
            "file:///get_files/get_files_12mx100/"
        )
    )
