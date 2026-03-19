"""
Integration tests for SnowflakeRestful.

Tests the backward-compatibility REST interface exposed via connection.rest.
These tests verify the contract that Snowflake CLI depends on.
Runs against both the universal driver and the reference (old) driver.
"""

from urllib.parse import urlparse

import pytest

from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


class TestSnowflakeRestfulProperties:
    """Test that connection.rest exposes expected properties after connecting."""

    def test_rest_is_not_none(self, connection):
        assert connection.rest is not None

    def test_token_is_non_empty_string(self, connection):
        """CLI depends on rest.token to authenticate REST API v2 calls."""
        token = connection.rest.token
        assert isinstance(token, str)
        assert len(token) > 0

    def test_master_token_is_non_empty_string(self, connection):
        """CLI reads rest.master_token for session token authentication."""
        master_token = connection.rest.master_token
        assert isinstance(master_token, str)
        assert len(master_token) > 0

    def test_server_url_is_valid(self, connection):
        """CLI uses rest.server_url to construct full URLs for fetch()."""
        server_url = connection.rest.server_url
        assert isinstance(server_url, str)
        assert server_url.startswith("https://") or server_url.startswith("http://")

    def test_server_url_contains_host_and_port(self, connection):
        """server_url should be in the form protocol://host:port."""
        parsed = urlparse(connection.rest.server_url)
        assert parsed.scheme in ("http", "https")
        assert parsed.hostname is not None
        assert len(parsed.hostname) > 0
        assert parsed.port is not None

    def test_host_is_non_empty_string(self, connection):
        host = connection.rest._host
        assert isinstance(host, str)
        assert len(host) > 0

    def test_host_matches_connection_host(self, connection):
        assert connection.rest._host == connection.host

    def test_port_is_valid(self, connection):
        port = connection.rest._port
        assert isinstance(port, int)
        assert port > 0

    def test_protocol_is_https_or_http(self, connection):
        assert connection.rest._protocol in ("https", "http")


class TestSnowflakeRestfulRequests:
    """Integration tests for request(), fetch(), and _token_request().

    Each test replicates an exact Snowflake CLI call pattern.
    """

    def test_request_rejects_unsupported_method(self, connection):
        """request() is an internal API used by Snowflake CLI.

        BD#17: New driver validates the HTTP method and raises ValueError.
        Old driver does not validate; it attempts the request and the server
        returns a 404 (HttpError).
        """
        if NEW_DRIVER_ONLY("BD#17"):
            with pytest.raises(ValueError, match="Unsupported HTTP method"):
                connection.rest.request(url="/test", method="delete", client="rest")
        elif OLD_DRIVER_ONLY("BD#17"):
            from snowflake.connector.errors import Error

            with pytest.raises(Error):
                connection.rest.request(url="/test", method="delete", client="rest")

    def test_request_monitoring_queries(self, connection):
        """Replicate the CLI's !queries command pattern.

        snowflake-cli/src/snowflake/cli/_plugins/sql/repl_commands.py:178
            ret = connection.rest.request(url=url, method="get", client="rest")
        """
        ret = connection.rest.request(url="/monitoring/queries?limit=1", method="get", client="rest")
        assert isinstance(ret, dict)
        assert "data" in ret
        assert "queries" in ret["data"]
        assert isinstance(ret["data"]["queries"], list)

    def test_fetch_rest_api_v2_databases(self, connection):
        """Replicate the CLI's RestApi.send_rest_request pattern.

        snowflake-cli/src/snowflake/cli/api/rest_api.py:70-102
        """
        import json

        rest = connection.rest
        full_url = f"{rest.server_url}/api/v2/databases/"
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json",
            "User-Agent": "PythonConnector/test",
        }
        ret = rest.fetch(
            method="get",
            full_url=full_url,
            headers=headers,
            token=rest.token,
            data=json.dumps({}),
            no_retry=True,
            raise_raw_http_failure=True,
            external_session_id=None,
        )
        # REST API v2 list databases returns an array of database objects
        assert isinstance(ret, list)
        assert len(ret) > 0
        assert "name" in ret[0]

    def test_token_request_issue(self, connection):
        """Replicate the CLI's SPCS image registry token issuance.

        snowflake-cli/src/snowflake/cli/_plugins/spcs/image_registry/manager.py:47
            token_data = self._conn._rest._token_request("ISSUE")
        """
        token_data = connection.rest._token_request("ISSUE")
        assert isinstance(token_data, dict)
        assert "data" in token_data
        data = token_data["data"]
        assert "sessionToken" in data
        assert isinstance(data["sessionToken"], str)
        assert len(data["sessionToken"]) > 0
        assert "validityInSecondsST" in data
        assert isinstance(data["validityInSecondsST"], int)
        assert data["validityInSecondsST"] > 0
