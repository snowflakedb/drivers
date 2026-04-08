"""
Integration tests for server-side error scenarios using Wiremock.

Tests verify that HTTP errors on query, login, and chunk download paths
are surfaced as proper PEP 249 exceptions. Each test starts a Wiremock
instance, configures mappings, and validates error handling behavior.
"""

import uuid

import pytest

from snowflake.connector.errors import (
    BadGatewayError,
    BadRequest,
    DatabaseError,
    ForbiddenError,
    GatewayTimeoutError,
    InternalServerError,
    MethodNotAllowed,
    OperationalError,
    RequestExceedMaxRetryError,
    ServiceUnavailableError,
    TooManyRequests,
)
from tests.utils import repo_root
from tests.wiremock_client import WiremockClient


# ---------------------------------------------------------------------------
# HTTP Errors on Query Execution Path
# ---------------------------------------------------------------------------


class TestQueryPathHTTPErrors:
    """Tests for HTTP errors during query execution via Wiremock."""

    def test_should_raise_bad_request_when_http_400_exhausts_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 400 for query requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_400.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                # Then BadRequest is raised
                with pytest.raises(BadRequest):
                    cursor.execute("SELECT 1")
            finally:
                cursor.close()

    def test_should_succeed_when_http_400_is_transient_then_200(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 400 once then HTTP 200
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_400_then_200.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                cursor.execute("SELECT 1")

                # Then No error is raised
                result = cursor.fetchone()
                assert result is not None
            finally:
                cursor.close()

    def test_should_raise_internal_server_error_when_http_500_exhausts_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 500 for query requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_500.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                # Then InternalServerError is raised
                with pytest.raises(InternalServerError):
                    cursor.execute("SELECT 1")
            finally:
                cursor.close()

    def test_should_raise_bad_gateway_error_when_http_502_exhausts_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 502 for query requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_502.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                # Then BadGatewayError is raised
                with pytest.raises(BadGatewayError):
                    cursor.execute("SELECT 1")
            finally:
                cursor.close()

    def test_should_raise_operational_error_when_http_503_exhausts_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 503 for query requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_503.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                # Then OperationalError is raised
                with pytest.raises(OperationalError):
                    cursor.execute("SELECT 1")
            finally:
                cursor.close()

    def test_should_raise_too_many_requests_when_http_429_exhausts_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 429 for query requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_429.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                # Then TooManyRequests is raised
                with pytest.raises(TooManyRequests):
                    cursor.execute("SELECT 1")
            finally:
                cursor.close()

    def test_should_succeed_when_http_408_is_transient_then_200(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 408 once then HTTP 200
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_http_408_then_200.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The client executes a query
                cursor.execute("SELECT 1")

                # Then No error is raised
                result = cursor.fetchone()
                assert result is not None
            finally:
                cursor.close()


# ---------------------------------------------------------------------------
# HTTP Errors on Login Path
# ---------------------------------------------------------------------------


class TestLoginPathHTTPErrors:
    """Tests for HTTP errors during authentication via Wiremock."""

    def test_should_raise_forbidden_error_for_http_403_on_login(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 403 for login requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("errors/login_http_403.json")

            # When The client attempts to connect
            # Then ForbiddenError is raised with message matching "Failed to connect to DB"
            with pytest.raises(ForbiddenError, match="(?i)failed to connect"):
                int_test_connection_factory(server_url=wiremock.http_url())

    def test_should_raise_bad_gateway_error_for_http_502_on_login(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 502 for login requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("errors/login_http_502.json")

            # When The client attempts to connect
            # Then BadGatewayError is raised with message matching "Service is unavailable"
            with pytest.raises(BadGatewayError, match="(?i)service is unavailable"):
                int_test_connection_factory(server_url=wiremock.http_url())

    def test_should_raise_service_unavailable_error_for_http_503_on_login(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 503 for login requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("errors/login_http_503.json")

            # When The client attempts to connect
            # Then ServiceUnavailableError is raised with message matching "Service is unavailable"
            with pytest.raises(ServiceUnavailableError, match="(?i)service is unavailable"):
                int_test_connection_factory(server_url=wiremock.http_url())


# ---------------------------------------------------------------------------
# HTTP Errors on Chunk Download Path
# ---------------------------------------------------------------------------


class TestChunkDownloadHTTPErrors:
    """Tests for HTTP errors during result set chunk downloads via Wiremock."""

    def test_should_raise_internal_server_error_for_http_500_during_chunk_download(
        self, int_test_connection_factory
    ):
        # Given A result set where wiremock returns HTTP 500 for chunk downloads
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_success_with_chunks.json")
            wiremock.add_mapping("errors/chunk_http_500.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                cursor.execute("SELECT 1")

                # When The client fetches result rows
                # Then InternalServerError is raised
                with pytest.raises(InternalServerError):
                    cursor.fetchall()
            finally:
                cursor.close()

    def test_should_raise_bad_gateway_error_for_http_502_during_chunk_download(
        self, int_test_connection_factory
    ):
        # Given A result set where wiremock returns HTTP 502 for chunk downloads
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_success_with_chunks.json")
            wiremock.add_mapping("errors/chunk_http_502.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                cursor.execute("SELECT 1")

                # When The client fetches result rows
                # Then BadGatewayError is raised
                with pytest.raises(BadGatewayError):
                    cursor.fetchall()
            finally:
                cursor.close()

    def test_should_raise_gateway_timeout_error_for_http_504_during_chunk_download(
        self, int_test_connection_factory
    ):
        # Given A result set where wiremock returns HTTP 504 for chunk downloads
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_success_with_chunks.json")
            wiremock.add_mapping("errors/chunk_http_504.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                cursor.execute("SELECT 1")

                # When The client fetches result rows
                # Then GatewayTimeoutError is raised
                with pytest.raises(GatewayTimeoutError):
                    cursor.fetchall()
            finally:
                cursor.close()

    def test_should_raise_too_many_requests_for_http_429_during_chunk_download(
        self, int_test_connection_factory
    ):
        # Given A result set where wiremock returns HTTP 429 for chunk downloads
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_success_with_chunks.json")
            wiremock.add_mapping("errors/chunk_http_429.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                cursor.execute("SELECT 1")

                # When The client fetches result rows
                # Then TooManyRequests is raised
                with pytest.raises(TooManyRequests):
                    cursor.fetchall()
            finally:
                cursor.close()

    def test_should_raise_method_not_allowed_for_http_405_during_chunk_download(
        self, int_test_connection_factory
    ):
        # Given A result set where wiremock returns HTTP 405 for chunk downloads
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/query_success_with_chunks.json")
            wiremock.add_mapping("errors/chunk_http_405.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                cursor.execute("SELECT 1")

                # When The client fetches result rows
                # Then MethodNotAllowed is raised
                with pytest.raises(MethodNotAllowed):
                    cursor.fetchall()
            finally:
                cursor.close()


# ---------------------------------------------------------------------------
# File Transfer via Wiremock
# ---------------------------------------------------------------------------


class TestFileTransferErrors:
    """Tests for file transfer error scenarios via Wiremock."""

    def test_should_raise_request_exceed_max_retry_error_when_put_upload_exhausts_storage_retries(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 503 for all storage PUT requests
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/put_storage_503.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The user executes a PUT command to an internal stage
                test_file_path = (
                    repo_root() / "tests" / "test_data" / "generated_test_data"
                    / "compression" / "test_data.csv"
                )
                put_command = f"PUT 'file://{test_file_path}' @TEST_STAGE"

                # Then RequestExceedMaxRetryError is raised
                with pytest.raises(RequestExceedMaxRetryError):
                    cursor.execute(put_command)
            finally:
                cursor.close()

    def test_should_succeed_put_after_storage_token_renewal(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns expired-token once then HTTP 200 for storage PUT
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/put_expired_token_then_200.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # When The user executes a PUT command to an internal stage
                test_file_path = (
                    repo_root() / "tests" / "test_data" / "generated_test_data"
                    / "compression" / "test_data.csv"
                )
                put_command = f"PUT 'file://{test_file_path}' @TEST_STAGE"
                cursor.execute(put_command)

                # Then The upload succeeds with no error
                result = cursor.fetchone()
                assert result is not None
            finally:
                cursor.close()

    def test_should_fall_back_to_inline_binds_when_executemany_stage_creation_fails(
        self, int_test_connection_factory
    ):
        # Given Wiremock returns HTTP 403 for stage creation requests
        # And CLIENT_STAGE_ARRAY_BINDING_THRESHOLD is set to trigger optimization
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("errors/stage_creation_403.json")
            connection = int_test_connection_factory(server_url=wiremock.http_url())
            cursor = connection.cursor()

            try:
                # Set binding threshold low to trigger stage-based optimization
                cursor.execute(
                    "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD=1"
                )

                table_name = f"test_fallback_{uuid.uuid4().hex[:8]}"
                cursor.execute(f"CREATE TEMPORARY TABLE {table_name} (val INT)")

                # When executemany is called with a large parameter list
                params = [[i] for i in range(10)]
                cursor.executemany(
                    f"INSERT INTO {table_name} VALUES (%s)", params
                )

                # Then The INSERT succeeds via inline binding fallback
                cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
                count = cursor.fetchone()[0]
                assert count == 10
            finally:
                cursor.close()


# ---------------------------------------------------------------------------
# Connection Timeout via Wiremock
# ---------------------------------------------------------------------------


class TestConnectionTimeoutErrors:
    """Tests for connection timeout scenarios via Wiremock."""

    def test_should_raise_operational_error_for_connection_timeout_during_login(
        self, int_test_connection_factory
    ):
        # Given Wiremock delays login response beyond the connection timeout
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("errors/login_timeout.json")

            # When The client attempts to connect with a short timeout
            # Then OperationalError is raised
            with pytest.raises(OperationalError):
                int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    login_timeout=2,
                )


# ---------------------------------------------------------------------------
# Async Query (real connection)
# ---------------------------------------------------------------------------


class TestAsyncQueryErrors:
    """Tests for async query error scenarios."""

    def test_should_raise_database_error_when_retrieving_results_for_failed_async_query(
        self, connection_factory
    ):
        # Given An async query that will fail on the server
        conn = connection_factory()
        cursor = conn.cursor()

        try:
            cursor.execute_async("SELEC 1")  # Intentionally malformed
            query_id = cursor.sfqid

            # When The user calls get_results_from_sfqid
            # Then DatabaseError is raised
            with pytest.raises(DatabaseError):
                cursor.get_results_from_sfqid(query_id)
        finally:
            cursor.close()
            conn.close()
