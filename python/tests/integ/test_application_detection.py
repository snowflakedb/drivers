"""Automatic ``application`` detection, compared across driver versions.

The legacy connector fills ``CLIENT_ENVIRONMENT.APPLICATION`` from ``SF_PARTNER``
or imported modules when ``application=`` is omitted. These tests open a
Wiremock-backed connection on both the universal and reference drivers and
assert the login payload.
"""

from __future__ import annotations

import json
import sys
import types

from snowflake.connector.constants import ENV_VAR_PARTNER
from tests.integ.telemetry._telemetry_helpers import jwt_private_key_params


def _login_application(wiremock) -> str:
    login_requests = wiremock.get_requests("/session/v1/login-request.*")
    assert login_requests, "Expected at least one login request"
    data = json.loads(login_requests[0]["body"])["data"]
    env = data["CLIENT_ENVIRONMENT"]
    if isinstance(env, str):
        env = json.loads(env)
    return env["APPLICATION"]


def _connect_and_read_application(int_test_connection_factory, wiremock, **kwargs) -> tuple[str, str]:
    """Open a JWT connection against Wiremock and return (property, login APPLICATION)."""
    wiremock.add_mapping("auth/login_success_jwt.json")
    connection = int_test_connection_factory(
        server_url=wiremock.http_url(),
        **jwt_private_key_params(),
        **kwargs,
    )
    try:
        return connection.application, _login_application(wiremock)
    finally:
        connection.close()


class TestApplicationDetection:
    def test_should_default_to_python_connector_when_nothing_is_detected(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # When a connection is opened with application omitted and no partner env/modules
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then both the property and the login payload use the driver default
        assert property_value == "PythonConnector"
        assert login_value == "PythonConnector"

    def test_should_use_sf_partner_env_var_as_application(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given SF_PARTNER is set and application= is omitted
        isolate_application_detection.setenv(ENV_VAR_PARTNER, "PartnerApp")
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then CLIENT_ENVIRONMENT.APPLICATION is the env value
        assert property_value == "PartnerApp"
        assert login_value == "PartnerApp"

    def test_should_detect_streamlit_imported_module(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given streamlit is importable and application= is omitted
        isolate_application_detection.setitem(sys.modules, "streamlit", types.ModuleType("streamlit"))
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then the application name is streamlit
        assert property_value == "streamlit"
        assert login_value == "streamlit"

    def test_should_detect_jupyter_notebook_when_all_jupyter_modules_are_imported(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given ipykernel, jupyter_core, and jupyter_client are all imported
        for name in ("ipykernel", "jupyter_core", "jupyter_client"):
            isolate_application_detection.setitem(sys.modules, name, types.ModuleType(name))
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then the application name is jupyter_notebook
        assert property_value == "jupyter_notebook"
        assert login_value == "jupyter_notebook"

    def test_should_not_detect_jupyter_when_only_ipykernel_is_imported(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given only ipykernel is imported (not jupyter_core / jupyter_client)
        isolate_application_detection.setitem(sys.modules, "ipykernel", types.ModuleType("ipykernel"))
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then detection does not treat this as a Jupyter notebook
        assert property_value == "PythonConnector"
        assert login_value == "PythonConnector"

    def test_should_detect_snowbooks_as_snowflake_notebook(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given snowbooks is imported
        isolate_application_detection.setitem(sys.modules, "snowbooks", types.ModuleType("snowbooks"))
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then the application name is snowflake_notebook
        assert property_value == "snowflake_notebook"
        assert login_value == "snowflake_notebook"

    def test_should_prefer_sf_partner_over_imported_modules(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given both SF_PARTNER and streamlit are present
        isolate_application_detection.setenv(ENV_VAR_PARTNER, "PartnerApp")
        isolate_application_detection.setitem(sys.modules, "streamlit", types.ModuleType("streamlit"))
        # When a connection is opened
        property_value, login_value = _connect_and_read_application(int_test_connection_factory, wiremock)
        # Then the env var wins
        assert property_value == "PartnerApp"
        assert login_value == "PartnerApp"

    def test_should_prefer_explicit_application_over_sf_partner(
        self, int_test_connection_factory, wiremock, isolate_application_detection
    ):
        # Given SF_PARTNER is set and the caller also passes application=
        isolate_application_detection.setenv(ENV_VAR_PARTNER, "PartnerApp")
        # When a connection is opened with an explicit application
        property_value, login_value = _connect_and_read_application(
            int_test_connection_factory, wiremock, application="MyApp"
        )
        # Then the explicit value wins
        assert property_value == "MyApp"
        assert login_value == "MyApp"
