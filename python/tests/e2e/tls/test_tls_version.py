"""Python e2e tests for tls/tls_version.feature (@python_e2e scenarios).

Mirrors the Core Rust and ODBC C++ coverage for TLS version negotiation.
All tests spin up a per-test WireMock JVM restricted to a single TLS version
via a JVM security-properties override.
"""

import pytest

from tests.utils import repo_root
from tests.wiremock_client import WiremockClient


_CA_PEM = str(repo_root() / "tests/wiremock/wiremock-ca.pem")


class TestTlsVersionEnforcement:
    @pytest.mark.skip_reference(reason="Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION")
    def test_should_negotiate_tls_when_the_server_offers_a_version_inside_the_window(self, int_test_connection_factory):
        # Given a TLS server that offers only TLS 1.3
        with WiremockClient(tls_version="tls13") as wm:
            wm.start()
            wm.add_mapping("auth/login_success_jwt.json")
            # And a client configured with min_tls_version tls12 and max_tls_version tls13
            connection_params = dict(
                server_url=wm.https_url(),
                custom_root_store_path=_CA_PEM,
                min_tls_version="tls12",
                max_tls_version="tls13",
            )
            # When a request is sent to the server
            conn = int_test_connection_factory(**connection_params)
            try:
                # Then the handshake succeeds
                assert conn is not None
            finally:
                conn.close(retry=False)

    @pytest.mark.skip_reference(reason="Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION")
    def test_should_fail_the_handshake_when_the_server_only_offers_a_version_below_the_minimum(
        self, int_test_connection_factory
    ):
        # Given a TLS server that offers only TLS 1.2
        with WiremockClient(tls_version="tls12") as wm:
            wm.start()
            # And a client configured with min_tls_version tls13
            connection_params = dict(
                server_url=wm.https_url(),
                custom_root_store_path=_CA_PEM,
                min_tls_version="tls13",
            )
            # When a request is sent to the server
            with pytest.raises(Exception):  # noqa: B017 — driver-specific class varies
                int_test_connection_factory(**connection_params)
            # Then the handshake fails
            wm.add_mapping("auth/login_success_jwt.json")
            permissive_conn = int_test_connection_factory(
                server_url=wm.https_url(),
                custom_root_store_path=_CA_PEM,
                min_tls_version="tls12",
                max_tls_version="tls13",
            )
            try:
                assert permissive_conn is not None, "Same TLS 1.2 server must succeed with a permissive window"
            finally:
                permissive_conn.close(retry=False)

    @pytest.mark.skip_reference(reason="Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION")
    def test_should_reject_the_configuration_when_the_minimum_exceeds_the_maximum(self, int_test_connection_factory):
        # Given settings with min_tls_version tls13 and max_tls_version tls12
        invalid_params = dict(min_tls_version="tls13", max_tls_version="tls12")
        # When the TLS configuration is built from settings
        with pytest.raises(Exception) as exc_info:
            int_test_connection_factory(**invalid_params)
        # Then a configuration error is returned
        assert "max_tls_version" in str(exc_info.value).lower()
