from runner.test_types import TestType

S3_TEST_DATA_12MX100 = "s3://sfc-eng-data/ecosystem/12Mx100/"
S3_TEST_DATA_12MX1000 = "s3://sfc-eng-data/ecosystem/12Mx1000/"
S3_TEST_DATA_1_2GX10 = "s3://sfc-eng-data/ecosystem/1.2Gx10/"


def test_put_files_12mx100(perf_test):
    """
    PUT test: Upload 100 files of 12MB each from local disk to temporary stage.
    Total: 1.2GB across 100 files
    """
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX100,
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=true"
        )
    )


def test_put_files_12mx1000(perf_test):
    """
    PUT test: Upload 1000 files of 12MB each from local disk to temporary stage.
    Total: 12GB across 1000 files
    """
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_12MX1000,
        setup_queries=[
            "CREATE TEMPORARY STAGE put_test_stage"
        ],
        sql_command=(
            "PUT file:///put_get_files/* @put_test_stage "
            "AUTO_COMPRESS=FALSE overwrite=true"
        )
    )


def test_put_files_1_2Gx10(perf_test):
    """
    PUT test: Upload 10 files of 1.2GB each from local disk to temporary stage.
    Total: 12GB across 10 files
    """
    perf_test(
        test_type=TestType.PUT_GET,
        s3_download_url=S3_TEST_DATA_1_2GX10,
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
        test_type=TestType.PUT_GET,
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
