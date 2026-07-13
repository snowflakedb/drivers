"""Integration tests for Connection.expired flag via Wiremock.

Covers both the positive case (flag becomes True after GS 390114) and the
negative space (flag stays False for every other error type).
"""

from contextlib import closing

import pytest


@pytest.mark.skip_reference(reason="Connection.expired RPC is universal-driver only")
class TestExpiredAfterGS390114:
    def test_should_set_expired_when_server_returns_390114(self, int_test_connection_factory, wiremock):
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("session/query_401_always.json")
        wiremock.add_mapping("session/token_request_gs_390114.json")

        with closing(int_test_connection_factory(server_url=wiremock.http_url())) as connection:
            try:
                with connection.cursor() as cur:
                    cur.execute("SELECT 1")
            except Exception:
                pass  # 401 → refresh → 390114 path always raises; we care about the flag

            assert connection.expired is True, "expired must be True after server returns GS 390114"


@pytest.mark.skip_reference(reason="Connection.expired RPC is universal-driver only")
class TestIsExpiredNotSet:
    """expired must stay False for every error type other than GS 390114."""

    def test_should_be_false_on_fresh_connection(self, int_test_connection_factory, wiremock):
        wiremock.add_mapping("auth/login_success_jwt.json")

        with closing(int_test_connection_factory(server_url=wiremock.http_url())) as connection:
            assert connection.expired is False, "expired must be False on a freshly connected session"

    def test_should_not_set_expired_on_query_500(self, int_test_connection_factory, wiremock):
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("session/query_500_always.json")

        with closing(int_test_connection_factory(server_url=wiremock.http_url())) as connection:
            try:
                with connection.cursor() as cur:
                    cur.execute("SELECT 1")
            except Exception:
                pass  # 500 always raises; we care about the flag

            assert connection.expired is False, "expired must stay False after a 500 server error"

    def test_should_not_set_expired_when_token_request_returns_session_gone(
        self, int_test_connection_factory, wiremock
    ):
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("session/query_401_always.json")
        wiremock.add_mapping("session/token_request_gs_390111.json")

        with closing(int_test_connection_factory(server_url=wiremock.http_url())) as connection:
            try:
                with connection.cursor() as cur:
                    cur.execute("SELECT 1")
            except Exception:
                pass  # 401 → refresh → 390111 path always raises; we care about the flag

            assert connection.expired is False, (
                "expired must stay False when token refresh returns 390111 (session_gone)"
            )
