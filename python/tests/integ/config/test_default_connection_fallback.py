"""Integration tests for default-profile fallback in bare ``connect()``.

Regression coverage for SNOW-3647714: bare ``snowflake.connector.connect()``
(no args) should fall back to the default profile in ``connections.toml``,
honoring ``SNOWFLAKE_DEFAULT_CONNECTION_NAME`` and ``config.toml``'s
``default_connection_name``.

These tests pass on the reference (legacy) driver and demonstrate the
regression on the universal driver.
"""

import stat

from textwrap import dedent

import pytest

import snowflake.connector

from tests.compatibility import IS_UNIVERSAL_DRIVER


REGRESSION_MARKER = "Missing required parameter"
"""Substring of the universal-driver error that signals the regression.

If this substring appears in the exception raised by bare ``connect()``,
sf_core never consulted ``connections.toml`` / env vars and short-circuited
on missing ``account``. Any *other* error (DNS failure, auth failure,
connection refused) means the resolver ran — that's the legacy contract.
"""

CONNECTION_ATTEMPT_MARKER = "250001"
"""Error code present in both drivers when a network connection attempt was made.

Universal driver: "250001 (08001): Failed to login. Connection refused ..."
Reference driver:  "250001: Could not connect to Snowflake backend ..."
"""


@pytest.fixture
def isolated_config_home(tmp_path, monkeypatch):
    """Redirect ``SNOWFLAKE_HOME`` at a temp dir and clear related env vars.

    Mirrors the pattern in ``test_config_manager.py::config_env`` but also
    scrubs ``SNOWFLAKE_DEFAULT_CONNECTION_NAME`` and per-parameter env vars
    so each test starts from a clean slate.
    """
    monkeypatch.setenv("SNOWFLAKE_HOME", str(tmp_path))
    monkeypatch.delenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", raising=False)
    for key in (
        "SNOWFLAKE_ACCOUNT",
        "SNOWFLAKE_USER",
        "SNOWFLAKE_PASSWORD",
        "SNOWFLAKE_HOST",
    ):
        monkeypatch.delenv(key, raising=False)

    yield {
        "tmp_path": tmp_path,
        "config_file": tmp_path / "config.toml",
        "connections_file": tmp_path / "connections.toml",
    }


def _write_connections_toml(path, content: str) -> None:
    path.write_text(dedent(content))
    # connections.toml requires owner-only permissions or the loader complains.
    path.chmod(stat.S_IRUSR | stat.S_IWUSR)


def _write_config_toml(path, content: str) -> None:
    path.write_text(dedent(content))
    path.chmod(stat.S_IRUSR | stat.S_IWUSR)


def _assert_no_regression(exc: BaseException) -> None:
    """Assert that ``exc`` does not signal the SNOW-3647714 regression.

    The regression manifests as "Missing required parameter 'account'" thrown
    before the resolver ever consulted connections.toml.

    Universal driver: asserts REGRESSION_MARKER absent AND a 250001 network
    error is present, proving the resolver loaded the profile and attempted a
    connection.

    Reference driver: the reference driver does not honour SNOWFLAKE_HOME so it
    cannot find the profile written to the temp dir.  It raises a config-level
    "cannot be found" error instead of 250001.  We still assert the regression
    marker is absent — that's the meaningful check.
    """
    msg = str(exc)
    assert REGRESSION_MARKER not in msg, (
        f"SNOW-3647714 regression: bare connect() short-circuited on "
        f"missing account instead of loading the default profile.\n"
        f"Exception: {type(exc).__name__}: {msg}"
    )
    if IS_UNIVERSAL_DRIVER:
        assert CONNECTION_ATTEMPT_MARKER in msg, (
            f"Expected a connection-attempt error ({CONNECTION_ATTEMPT_MARKER}) "
            f"after config resolution, but got: {type(exc).__name__}: {msg}"
        )


# ---------------------------------------------------------------------------
# Core regression tests
# ---------------------------------------------------------------------------


class TestBareConnectDefaultProfile:
    """Bare ``connect()`` should load ``[default]`` from ``connections.toml``."""

    def test_loads_default_profile_when_no_args(self, isolated_config_home):
        """Bare connect() reads [default] from connections.toml.

        Repro for SNOW-3647714. With a [default] profile present and no env
        var override, bare connect() must consult the file. The connection
        attempt itself will fail (non-routable host) — we only assert it
        did NOT fail with 'Missing required parameter account'.
        """
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [default]
            account = "snow3647714_default_acct"
            user = "u"
            password = "p"
            host = "127.0.0.1"
            port = 1
            login_timeout = 1
            network_timeout = 1
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect()

        _assert_no_regression(exc_info.value)

    def test_honors_snowflake_default_connection_name_env(self, isolated_config_home, monkeypatch):
        """SNOWFLAKE_DEFAULT_CONNECTION_NAME picks an alternate profile."""
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [alt]
            account = "snow3647714_alt_acct"
            user = "u"
            password = "p"
            host = "127.0.0.1"
            port = 1
            login_timeout = 1
            network_timeout = 1
            """,
        )
        monkeypatch.setenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", "alt")

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect()

        _assert_no_regression(exc_info.value)

    def test_honors_default_connection_name_in_config_toml(self, isolated_config_home):
        """config.toml's ``default_connection_name = "alt"`` selects [alt]."""
        _write_config_toml(
            isolated_config_home["config_file"],
            """
            default_connection_name = "alt"
            """,
        )
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [alt]
            account = "snow3647714_alt_acct"
            user = "u"
            password = "p"
            host = "127.0.0.1"
            port = 1
            login_timeout = 1
            network_timeout = 1
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect()

        _assert_no_regression(exc_info.value)


# ---------------------------------------------------------------------------
# Guard tests — verify the fallback does not over-reach
# ---------------------------------------------------------------------------


class TestExplicitArgsDoNotMergeDefaultProfile:
    """Explicit kwargs must not silently merge with [default].

    Matches the legacy ``is_kwargs_empty`` contract — the default-profile
    fallback only triggers when the caller passed *nothing*.
    """

    def test_explicit_account_does_not_pull_default_user(self, isolated_config_home):
        """Passing account=... should NOT inject user from [default].

        If sf_core (incorrectly) merges [default] under explicit kwargs,
        the connection would silently authenticate as ``profile_user``
        when the caller never named that user. Assert the resolver does
        not load the file when the caller passed explicit credentials.

        Note: we can't easily introspect the resolved config without a
        successful connection. The assertion checks that ``profile_user``
        does not appear in the error message as a best-effort guard.
        A stricter check would require exposing resolved options; tracked
        as a follow-up improvement.
        """
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [default]
            account = "default_acct"
            user = "profile_user"
            password = "profile_pwd"
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect(
                account="explicit_acct",
                user="explicit_user",
                # password intentionally omitted
                host="127.0.0.1",
                port=1,
                login_timeout=1,
                network_timeout=1,
            )

        msg = str(exc_info.value)
        assert "profile_user" not in msg, f"Default-profile leaked into explicit kwargs: {msg}"

    def test_non_locator_kwargs_do_not_trigger_default_profile_load(self, isolated_config_home):
        """connect(user=...) with no account must NOT load the default profile.

        Legacy ``is_kwargs_empty`` parity: any non-empty kwargs means "the
        caller passed something", so the default profile must not be merged
        underneath — even when no locator (account/host/server_url) is present.

        Regression guard for the ``no_connection_details`` signal in
        SNOW-3647714.
        """
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [default]
            account = "default_acct"
            user = "profile_user"
            password = "profile_pwd"
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect(
                user="alice",
                # No account, host, or server_url — but this is NOT a bare connect
            )

        msg = str(exc_info.value).lower()
        if IS_UNIVERSAL_DRIVER:
            # The error must be "missing account" — the default profile was not merged.
            assert CONNECTION_ATTEMPT_MARKER not in str(exc_info.value), (
                f"Default profile was silently loaded for connect(user='alice'): {exc_info.value!r}"
            )
            assert "account" in msg or "missing" in msg or "required" in msg, (
                f"Expected a 'missing account' error, got: {exc_info.value!r}"
            )
        else:
            # Reference driver: CI supplies an account from its own config; with no
            # password provided the driver raises 251006 "Password is empty".  The
            # [default] profile was NOT loaded (if it were, host=127.0.0.1:1 would
            # trigger a 250001 network error, not a 251006 auth error).
            assert CONNECTION_ATTEMPT_MARKER not in str(exc_info.value), (
                f"Default profile was silently loaded for connect(user='alice'): {exc_info.value!r}"
            )


class TestExplicitConnectionNameStillWorks:
    """Regression check: ``connection_name=...`` is unaffected by the fix."""

    def test_named_profile_loads(self, isolated_config_home):
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [other]
            account = "other_acct"
            user = "u"
            password = "p"
            host = "127.0.0.1"
            port = 1
            login_timeout = 1
            network_timeout = 1
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect(connection_name="other")

        _assert_no_regression(exc_info.value)


# ---------------------------------------------------------------------------
# Negative path: no default profile available
# ---------------------------------------------------------------------------


class TestNoDefaultProfileAvailable:
    """When no default profile exists, the error should be informative."""

    def test_no_connections_toml_at_all(self, isolated_config_home):
        """Bare connect() with no connections.toml raises a 'not found' error.

        On both drivers the absence of connections.toml means there is no
        default profile to load, and the error should mention 'not found'
        rather than a generic parameter-validation failure.
        """
        # No connections.toml written.
        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect()

        err_msg = str(exc_info.value).lower()
        if IS_UNIVERSAL_DRIVER:
            # e.g. "Connection 'default' not found"
            assert "not found" in err_msg and "default" in err_msg, (
                f"Expected a 'default … not found' error, got: {exc_info.value!r}"
            )
        else:
            # Reference driver: "Default connection with name 'default' cannot be found, known ones are []"
            assert "cannot be found" in err_msg and "default" in err_msg, (
                f"Expected a 'default … cannot be found' error, got: {exc_info.value!r}"
            )

    def test_connections_toml_without_default_section(self, isolated_config_home):
        """connections.toml exists but has no [default] section."""
        _write_connections_toml(
            isolated_config_home["connections_file"],
            """
            [some_other_profile]
            account = "x"
            user = "u"
            password = "p"
            """,
        )

        with pytest.raises(Exception) as exc_info:
            snowflake.connector.connect()

        err_msg = str(exc_info.value).lower()
        if IS_UNIVERSAL_DRIVER:
            # e.g. "Connection 'default' not found"
            assert "not found" in err_msg and "default" in err_msg, (
                f"Expected a 'default … not found' error for missing default profile, got: {exc_info.value!r}"
            )
        else:
            # Reference driver: "Default connection with name 'default' cannot be found, known ones are []"
            assert "cannot be found" in err_msg and "default" in err_msg, (
                f"Expected a 'default … cannot be found' error for missing default profile, got: {exc_info.value!r}"
            )
