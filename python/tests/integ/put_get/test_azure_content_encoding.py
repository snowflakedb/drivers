from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.utils import repo_root


def test_azure_put_does_not_send_content_encoding(
    int_test_connection_factory, wiremock, old_driver_azure_routes_to_wiremock
):
    """BD#32 reference test — Azure stage PUT and the Content-Encoding header.

    The legacy snowflake-connector-python sends ``Content-Encoding: utf-8`` on
    single-shot Azure PUT (the upload-chunk path in ``azure_storage_client.py``)
    and ``x-ms-blob-content-encoding: utf-8`` on the block-list commit (the
    final PUT that finalizes a multi-block upload of a large file).
    ``utf-8`` is invalid as an HTTP content coding (RFC 9110 §8.4 lists only
    gzip, br, deflate, zstd, identity; ``utf-8`` is a charset, not a coding).
    Snowflake JDBC and libsfclient both omit the header entirely. The new
    universal driver matches JDBC/libsfclient.
    """
    # Given Snowflake client is logged in, with PUT routed through wiremock
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping(
        "put_get/azure_put_capture.json",
        placeholders={"{{WIREMOCK_HTTP_URL}}": wiremock.http_url()},
    )
    # And connection.close() cleanup mappings so teardown doesn't 404-noisily
    wiremock.add_mapping("telemetry/telemetry_send_success.json")
    wiremock.add_mapping("session/logout_success.json")
    with int_test_connection_factory(server_url=wiremock.http_url()) as connection, connection.cursor() as cursor:
        # When File is uploaded to a wiremock-routed Azure stage
        test_file_path = repo_root() / "tests" / "test_data" / "generated_test_data" / "basic" / "test_data.csv"
        cursor.execute(f"PUT 'file://{test_file_path}' @AZURE_TEST_STAGE OVERWRITE=TRUE")

        # Then wiremock captured the Azure blob PUT — pull it from the journal
        captured = wiremock.get_requests("/test-container/prefix/.*")
        put_requests = [r for r in captured if r["method"] == "PUT"]
        assert len(put_requests) >= 1, "expected at least one Azure blob PUT in wiremock journal"
        put_entry = put_requests[0]

        # HTTP header names are case-insensitive on the wire but wiremock
        # preserves the original case. Normalize before checking.
        headers = {k.lower(): v for k, v in put_entry["headers"].items()}

        # And the BD#32 contract holds for the driver under test.
        if NEW_DRIVER_ONLY("BD#32"):
            assert "content-encoding" not in headers, (
                f"new driver must not set Content-Encoding on Azure PUT (got {headers.get('content-encoding')!r})"
            )
            assert "x-ms-blob-content-encoding" not in headers, (
                f"new driver must not set x-ms-blob-content-encoding "
                f"(got {headers.get('x-ms-blob-content-encoding')!r})"
            )

        if OLD_DRIVER_ONLY("BD#32"):
            # Single-shot upload path (small file, autoCompress off, threshold
            # not exceeded → one PUT, no block-list commit) sets the transport
            # variant. The metadata variant (x-ms-blob-content-encoding) only
            # appears on the multipart-commit codepath.
            assert headers.get("content-encoding") == "utf-8", (
                f"old driver should send Content-Encoding: utf-8 on single-shot "
                f"Azure PUT; got {headers.get('content-encoding')!r}"
            )
