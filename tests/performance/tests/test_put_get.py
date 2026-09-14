import pytest

from runner.test_types import PerfTestType

S3_TEST_DATA_12MX100 = "s3://sfc-eng-data/ecosystem/12Mx100/"
S3_TEST_DATA_1_2G = "s3://sfc-eng-data/ecosystem/1.2Gx10/"


def test_put_files_12mx100(perf_test):
    """
    PUT test: Upload 100 files of 12MB each from local disk to temporary stage.
    Total: 1.2GB across 100 files
    """
    perf_test(
        test_type=PerfTestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX100,
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=true"
        )
    )


@pytest.mark.iterations(3)
def test_put_file_1_2g(perf_test):
    """
    PUT test: Upload one 1.2GB file from local disk to temporary stage.
    """
    perf_test(
        test_type=PerfTestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_1_2G,
        s3_max_files=1,
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=true"
        )
    )


def test_get_files_12mx100(perf_test):
    """
    GET test: Download 100 files of 12MB each from temporary stage to local disk.
    Total: 1.2GB across 100 files
    """
    perf_test(
        test_type=PerfTestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX100,
        
        setup_queries=[
            "CREATE TEMPORARY STAGE get_test_stage",
            "PUT file:///put_get_files/* @get_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=false"
        ],
        sql_command=(
            "GET @get_test_stage "
            "file:///get_files/get_files_12mx100/"
        )
    )


@pytest.mark.iterations(3)
def test_get_file_1_2g(perf_test):
    """
    GET test: Download one 1.2GB file from temporary stage to local disk.
    """
    perf_test(
        test_type=PerfTestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_1_2G,
        s3_max_files=1,
        setup_queries=[
            "CREATE TEMPORARY STAGE get_test_stage",
            "PUT file:///put_get_files/* @get_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=false"
        ],
        sql_command=(
            "GET @get_test_stage "
            "file:///get_files/get_file_1_2g/"
        )
    )
