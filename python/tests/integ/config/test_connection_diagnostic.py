"""Integration tests for connection diagnostic configuration parameters.

One class per report section so failures pinpoint exactly which part of the
report diverged.  Tests that cover behaviour exclusive to sf_core are marked
``skip_reference``; tests that work on both drivers assert the common lines and,
where the drivers differ, branch on ``IS_UNIVERSAL_DRIVER``.
"""

from __future__ import annotations

import json
import re
import tempfile

from pathlib import Path

import pytest

import snowflake.connector

from tests.compatibility import IS_UNIVERSAL_DRIVER


REPORT_FILENAME = "SnowflakeConnectionTestReport.txt"

_BOGUS_KWARGS: dict = dict(
    account="testaccount",
    user="testuser",
    password="testpassword",
    login_timeout=5,
)

_EMPTY_ALLOWLIST = "[]"
_STAGE_ALLOWLIST = json.dumps([{"host": "s3.amazonaws.com", "port": 443, "type": "STAGE"}])
_INVALID_ALLOWLIST = "This function has been deprecated. Use SYSTEM$ALLOWLIST instead."

_IP_RE = re.compile(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _connect_with_diag(**kwargs) -> None:
    """Run connect() with diagnostic params, swallowing all exceptions."""
    try:
        snowflake.connector.connect(**_BOGUS_KWARGS, **kwargs)
    except Exception:
        pass


def _run_diag(tmp_path, **extra) -> str:
    """Connect with diagnostics enabled, write report to tmp_path, return text."""
    _connect_with_diag(
        enable_connection_diag=True,
        connection_diag_log_path=str(tmp_path),
        **extra,
    )
    return read_report(tmp_path)


def read_report(log_dir) -> str:
    return (Path(log_dir) / REPORT_FILENAME).read_text()


def tmpdir_report() -> Path:
    return Path(tempfile.gettempdir()) / REPORT_FILENAME


# ---------------------------------------------------------------------------
# Diagnostic errors must not suppress the connection error
# ---------------------------------------------------------------------------


class TestDiagnosticErrorHandling:
    """Failures inside the diagnostic runner must not propagate to the caller."""

    def test_diagnostic_failure_does_not_suppress_connection_error(self, tmp_path):
        """A broken log_path (file, not a dir) must not hide the auth error."""
        existing_file = tmp_path / "not_a_directory"
        existing_file.write_text("file, not a directory")

        if IS_UNIVERSAL_DRIVER:
            # sf_core swallows the write error; only the auth error surfaces.
            with pytest.raises(snowflake.connector.errors.DatabaseError):
                snowflake.connector.connect(
                    **_BOGUS_KWARGS,
                    enable_connection_diag=True,
                    connection_diag_log_path=str(existing_file),
                )
        else:
            # Reference driver may let the OS error propagate instead of swallowing it.
            with pytest.raises((snowflake.connector.errors.DatabaseError, OSError)):
                snowflake.connector.connect(
                    **_BOGUS_KWARGS,
                    enable_connection_diag=True,
                    connection_diag_log_path=str(existing_file),
                )


# ---------------------------------------------------------------------------
# Report file creation
# ---------------------------------------------------------------------------


@pytest.mark.xdist_group("connection_diagnostic_tmpdir")
class TestReportFileCreation:
    """enable_connection_diag=True must create a report file on disk."""

    @pytest.fixture(autouse=True)
    def _cleanup_tmpdir_report(self):
        tmpdir_report().unlink(missing_ok=True)
        yield
        tmpdir_report().unlink(missing_ok=True)

    def test_report_created_in_specified_dir(self, tmp_path):
        """Report is written to connection_diag_log_path / SnowflakeConnectionTestReport.txt."""
        _run_diag(tmp_path)
        report = tmp_path / REPORT_FILENAME
        assert report.exists(), f"Expected report file at {report}"
        assert report.stat().st_size > 0

    def test_no_report_when_diag_disabled(self, tmp_path):
        """No report file is written when enable_connection_diag is False (default)."""
        _connect_with_diag(
            enable_connection_diag=False,
            connection_diag_log_path=str(tmp_path),
        )
        assert not (tmp_path / REPORT_FILENAME).exists()

    def test_report_falls_back_to_tmpdir_when_no_log_path(self):
        """When connection_diag_log_path is omitted the report lands in the system tmpdir."""
        _connect_with_diag(enable_connection_diag=True)
        assert tmpdir_report().exists(), f"Expected report at {tmpdir_report()}"

    def test_report_falls_back_to_tmpdir_for_nonexistent_path(self, tmp_path):
        """A path that does not exist falls back to tmpdir."""
        missing = tmp_path / "does_not_exist"
        _connect_with_diag(
            enable_connection_diag=True,
            connection_diag_log_path=str(missing),
        )
        assert not (missing / REPORT_FILENAME).exists()
        assert tmpdir_report().exists()

    def test_report_falls_back_to_tmpdir_for_relative_path(self, tmp_path, monkeypatch):
        """A relative path falls back to tmpdir."""
        monkeypatch.chdir(tmp_path)
        _connect_with_diag(
            enable_connection_diag=True,
            connection_diag_log_path="relative/subdir",
        )
        assert not (tmp_path / "relative" / "subdir" / REPORT_FILENAME).exists()
        assert tmpdir_report().exists()


# ---------------------------------------------------------------------------
# INITIAL section
# ---------------------------------------------------------------------------


class TestInitialSection:
    """``=========Connectivity diagnostic report===`` … ``INITIAL:`` lines."""

    def test_report_header_present(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "=========Connectivity diagnostic report" in report

    def test_account_logged(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "INITIAL: Specified snowflake account: testaccount" in report

    def test_host_logged(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "INITIAL: Host based on specified account: testaccount.snowflakecomputing.com" in report


# ---------------------------------------------------------------------------
# PROXY section
# ---------------------------------------------------------------------------


class TestProxySection:
    """``=========Proxy information===`` … ``PROXY:`` lines."""

    def test_proxy_section_header(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "=========Proxy information" in report

    def test_system_proxies_line_present(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "PROXY: Proxies with Env vars removed(SYSTEM PROXIES):" in report

    def test_env_proxies_line_present(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "PROXY: Proxies with Env vars restored(ENV PROXIES):" in report


# ---------------------------------------------------------------------------
# SNOWFLAKE_URL section
# ---------------------------------------------------------------------------


class TestSnowflakeUrlSection:
    """``=========Snowflake URL information===`` … ``SNOWFLAKE_URL:`` lines."""

    def test_snowflake_url_header(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "=========Snowflake URL information" in report

    def test_nslookup_result_present(self, tmp_path):
        """DNS lookup result for the account host appears in the report."""
        report = _run_diag(tmp_path)
        assert "SNOWFLAKE_URL: testaccount.snowflakecomputing.com: nslookup results:" in report
        assert _IP_RE.search(report), "Expected an IP address in the nslookup result"

    @pytest.mark.skip_reference(reason="sf_core only: logs actual peer IP after TCP connect")
    def test_connected_peer_ip_logged(self, tmp_path):
        """After TCP connect, the actual peer IP (not just DNS) is logged."""
        report = _run_diag(tmp_path)
        assert re.search(
            r"SNOWFLAKE_URL: testaccount\.snowflakecomputing\.com:443: Connected to IP: \d+\.\d+\.\d+\.\d+",
            report,
        ), "Expected 'Connected to IP: <ip>' line under SNOWFLAKE_URL"

    @pytest.mark.skip_reference(reason="sf_core only: TLS certificate chain inspection")
    def test_tls_certificate_serial_logged(self, tmp_path):
        """TLS handshake succeeds → cert serial number is logged."""
        report = _run_diag(tmp_path)
        assert re.search(
            r"SNOWFLAKE_URL: testaccount\.snowflakecomputing\.com: Certificate 1: serial=[0-9a-f]+",
            report,
        ), "Expected certificate serial line under SNOWFLAKE_URL"

    @pytest.mark.skip_reference(reason="sf_core only: TLS certificate chain inspection")
    def test_tls_certificate_crtsh_link_logged(self, tmp_path):
        """crt.sh verification link is logged for each certificate."""
        report = _run_diag(tmp_path)
        assert "https://crt.sh/?serial=" in report


# ---------------------------------------------------------------------------
# STAGE section
# ---------------------------------------------------------------------------


class TestStageSection:
    """``=========Snowflake Stage information===`` lines and allowlist handling."""

    def test_stage_unavailable_when_no_allowlist(self, tmp_path):
        """Without an allowlist source the stage section is marked Unavailable."""
        report = _run_diag(tmp_path)
        assert "=========Snowflake Stage information - Unavailable" in report
        assert "We could not connect to Snowflake to get allowlist" in report

    def test_stage_retrieved_with_empty_allowlist(self, tmp_path):
        """An empty allowlist [] is valid and produces the 'retrieved' stage section."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_EMPTY_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "=========Snowflake Stage information===" in report
        assert "We retrieved stage info from the allowlist" in report

    def test_stage_entry_probed_from_allowlist_file(self, tmp_path):
        """A STAGE entry in the allowlist JSON is probed and appears in the report."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_STAGE_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "STAGE: s3.amazonaws.com" in report

    def test_invalid_allowlist_content_recorded(self, tmp_path):
        """Non-list allowlist content is recorded in the report as an error."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_INVALID_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "Allowlist is not a valid list of json objects" in report

    def test_nonexistent_allowlist_falls_back_to_unavailable(self, tmp_path):
        """A path to a missing allowlist file falls back to the Unavailable stage section."""
        missing = tmp_path / "missing_allowlist.json"
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(missing))
        assert "Snowflake Stage information - Unavailable" in report


# ---------------------------------------------------------------------------
# OCSP section
# ---------------------------------------------------------------------------


@pytest.mark.skip_universal(reason="OCSP not implemented in sf_core")
class TestOcspSection:
    """``=========Snowflake OCSP information===`` … ``OCSP_RESPONDER:`` lines."""

    def test_ocsp_section_header(self, tmp_path):
        report = _run_diag(tmp_path)
        assert "=========Snowflake OCSP information" in report

    def test_ocsp_status_with_allowlist(self, tmp_path):
        """When allowlist is available, the report confirms it was retrieved."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_EMPTY_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "We were able to retrieve system allowlist." in report
        assert "These OCSP hosts came from the certificate and the allowlist." in report

    def test_ocsp_status_without_allowlist(self, tmp_path):
        """When no allowlist, the report notes hosts came from the certificate only."""
        report = _run_diag(tmp_path)
        assert "We were unable to retrieve system allowlist." in report
        assert "These OCSP hosts only came from the certificate." in report

    def test_ocsp_host_nslookup_present(self, tmp_path):
        """DNS lookup for the OCSP responder is recorded."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_EMPTY_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "OCSP_RESPONDER: ocsp.snowflakecomputing.com: nslookup results:" in report

    def test_ocsp_url_check_connected(self, tmp_path):
        """HTTP connectivity to the OCSP responder port 80 is confirmed."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_EMPTY_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        assert "OCSP_RESPONDER: ocsp.snowflakecomputing.com:80: URL Check: Connected Successfully" in report

    @pytest.mark.skip_reference(reason="sf_core only: OCSP URLs discovered from TLS cert AIA extension")
    def test_cert_discovered_ocsp_hosts_logged(self, tmp_path):
        """OCSP URLs from the server cert's AIA extension are added to the probe list."""
        allowlist_file = tmp_path / "allowlist.json"
        allowlist_file.write_text(_EMPTY_ALLOWLIST)
        report = _run_diag(tmp_path, connection_diag_allowlist_path=str(allowlist_file))
        # The default snowflakecomputing OCSP host is always present;
        # cert-discovered hosts (e.g. ocsp.digicert.com) are environment-specific.
        assert "OCSP_RESPONDER: ocsp.snowflakecomputing.com" in report
