"""Integration test: Connection.expired becomes True after server returns GS 390114.

Proves the full Python → sf_core → HTTP chain:
  login success → query 401 → token-request → 390114 → expired flag set.
"""

import pytest


@pytest.mark.skip_reference(reason="Connection.expired RPC is universal-driver only")
class TestExpiredAfterGS390114:
    def test_should_set_expired_when_server_returns_390114(self, int_test_connection_factory, wiremock):
        wiremock.add_mapping("auth/login_success_jwt.json")
        wiremock.add_mapping("session/query_401_always.json")
        wiremock.add_mapping("session/token_request_gs_390114.json")

        connection = int_test_connection_factory(server_url=wiremock.http_url())
        try:
            connection.cursor().execute("SELECT 1")
        except Exception:
            pass  # 401 → refresh → 390114 path always raises; we care about the flag

        assert connection.expired is True, "expired must be True after server returns GS 390114"
