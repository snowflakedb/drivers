import pytest

from tests.wiremock_client import WiremockClient
from tests.utils import repo_root
from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


def test_should_return_error_for_unsupported_compression_type(int_test_connection_factory):
    # Given Snowflake client is logged in
    with WiremockClient().start() as wiremock:
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("put_get/put_unsupported_compression_type.json")
        connection = int_test_connection_factory(server_url=wiremock.http_url())
        cursor = connection.cursor()
        try:
            assert cursor is not None
            
            # And File compressed with unsupported format
            test_file_path = (
                repo_root()
                / "tests"
                / "test_data"
                / "generated_test_data"
                / "compression"
                / "test_data.csv.xz"
            )
            
            # When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
            put_command = (
                f"PUT 'file://{test_file_path}' "
                f"@TEST_STAGE SOURCE_COMPRESSION=AUTO_DETECT"
            )
            
            # Then Unsupported compression error is thrown
            with pytest.raises(Exception) as exc_info:
                cursor.execute(put_command)
            
            if NEW_DRIVER_ONLY("BD#4"):
                assert "Unsupported compression type" in str(exc_info.value)
            
            if OLD_DRIVER_ONLY("BD#4"):
                assert "253007" in str(exc_info.value)
                assert "Feature is not supported" in str(exc_info.value)
        finally:
            cursor.close()