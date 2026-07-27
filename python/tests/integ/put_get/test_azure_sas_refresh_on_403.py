"""Integration tests for Azure SAS refresh on 403 (wiremock-driven).

Pins the Python Gherkin scenarios in
``tests/definitions/shared/put_get/azure_sas_refresh_on_403.feature``.

These tests are deterministic and require NO live Azure account: like the
other connectors, they force the 403 with a mock rather than expiring a real
SAS. Forcing the 403 over the wire with wiremock exercises the real HTTP
client + Azure-error parsing, not a mock below the network.

How the mock forces the refresh
-------------------------------
The universal driver's Rust core does all storage HTTP. On any Azure 403
(any-403 trigger, HTTP/2-safe) the core re-issues the original
PUT/GET SQL to GS to obtain a fresh ``stageInfo`` (creds), then retries the
failing HTTP attempt with the new SAS (``SnowflakeStageInfoRefresher`` in
``sf_core/src/apis/database_driver_v1/query.rs``). A single wiremock
instance plays BOTH roles at once:

1. **Snowflake GS** — serves the PUT-command response. Because the core
   honours a scheme-prefixed ``stageInfo.endPoint``
   (``azure_transfer.rs::build_azure_url``, built for Azurite/mock servers),
   we point ``endPoint`` at wiremock's own URL so every Azure blob request
   lands back on the same mock. Wiremock *scenario state* returns an
   ``sig=expiredsig`` SAS on the first GS call and an ``sig=freshsig`` SAS on
   the refresh re-issue.
2. **Azure Blob storage** — the blob PUT is matched on the ``sig`` query
   parameter: ``expiredsig`` → 403, ``freshsig`` → 201.

Observability
-------------
Core ``tracing`` events are bridged through the Rust→Python FFI log bridge
(``sf_core/src/logging/callback_layer.rs``) to the
``snowflake.connector._core`` logger; pytest ``caplog`` intercepts them
directly (it propagates to root by default). The callback layer folds every
tracing field into the message string (``message`` verbatim, then
`` name=value`` per field), so a terminal warn renders as e.g.
``Azure PUT failed terminally with 403 after SAS refresh status=403
code="AuthorizationPermissionMismatch" url_redacted=...``.
Severity tracks the OUTCOME:

- recovered 403  → debug breadcrumb only, NO warn
- refresh mechanism itself fails → error naming the refresh failure
- terminal non-token 403 (refreshed SAS also rejected) → warn carrying
  status / Azure ``<Code>`` / SAS-redacted URL

Scope of this file
------------------
Covers the three PUT-path scenarios that wiremock can force without a code
seam. The download (GET) refresh scenario and the per-chunk-resume parity
scenario remain core-only: Azure download is a single streaming GET
(``azure_transfer.rs`` "NOT per-chunk" note), so the per-chunk-resume
discriminator is a core/JDBC-comparison concern, and the GET refresh is
already proven at core level by
``azure_streaming_get_redrives_whole_get_with_refreshed_sas_on_403``. A
Python wiremock GET test (serving blob bytes + echoing the client's local
download dir) is a fast-follow. The 10-minute coalesce scenario is core-only.
"""

from __future__ import annotations

import base64
import logging

import pytest
import requests

from snowflake.connector.errors import Error
from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from tests.utils import repo_root


# Logger name the Rust→Python FFI bridge emits Core tracing events to. Same
# logger asserted on by python/tests/e2e/session/test_close.py.
CORE_LOGGER = "snowflake.connector._core"

# SAS signature markers wiremock scenario-state swaps in: the first GS call
# serves the expired SAS, the refresh re-issue serves the fresh one. The blob
# PUT matches on the sig query parameter (expired → 403, fresh → 201).
_EXPIRED_SIG = "sig=expiredsig"
_FRESH_SIG = "sig=freshsig"

# Azure blob PUT path (test-container/prefix/<name>) and GS query-request path,
# as journalled by wiremock; used to filter captured requests.
_AZURE_BLOB_PATH_RE = "/test-container/prefix/.*"
_GS_QUERY_PATH_RE = "/queries/v1/query-request.*"

# Placeholder the parameterized mappings carry; substituted with wiremock's own
# URL at add_mapping time so every Azure blob request lands back on the mock.
_WIREMOCK_URL_PLACEHOLDER = "{{WIREMOCK_HTTP_URL}}"

# PUT result-row schema: status lives at column index 6 and reads "UPLOADED" on
# success. Asserting on this loudly makes a result-shape change fail here.
_STATUS_COL = 6
_STATUS_UPLOADED = "UPLOADED"


def _assert_all_uploaded(put_results) -> None:
    """Assert every PUT result row carries an UPLOADED status at _STATUS_COL.

    Asserts loudly on schema first (every row must be long enough to carry the
    status column) so a result-shape change fails here instead of silently
    skipping short rows.
    """
    assert put_results, f"PUT returned no result rows; got {put_results!r}"
    assert all(len(row) > _STATUS_COL for row in put_results), (
        f"every PUT result row must carry the status column at index {_STATUS_COL}; got rows={put_results!r}"
    )
    statuses = [row[_STATUS_COL] for row in put_results]
    assert all(s == _STATUS_UPLOADED for s in statuses), (
        f"PUT must report {_STATUS_UPLOADED} across the forced 403 + SAS refresh; got rows={put_results!r}"
    )


# Test payload: the small CSV the put_get test-data generator produces. Uploaded
# uncompressed (the mapping's stageInfo carries autoCompress=false), so the Azure
# blob PUT lands at /test-container/prefix/<name> with the SAS appended as the
# query string.
_TEST_DATA_REL = ("tests", "test_data", "generated_test_data", "basic", "test_data.csv")

# Distinct stage name so our mappings only ever match our own PUT (never another
# test's @AZURE_TEST_STAGE upload). Must match the bodyPatterns in the three
# azure_sas_refresh_on_403_*.json mappings.
_PUT_TARGET = "@AZURE_SAS_REFRESH_STAGE"


def _put_command() -> str:
    test_file_path = repo_root().joinpath(*_TEST_DATA_REL)
    # PUT does not support IDENTIFIER(?)/bind params for file paths; test_file_path
    # derives from the hardcoded _TEST_DATA_REL constant — no user-controlled input.
    return f"PUT 'file://{test_file_path}' {_PUT_TARGET} OVERWRITE=TRUE"


def _azure_puts(wiremock) -> list[dict]:
    """All Azure blob PUTs wiremock journalled for this test."""
    captured = wiremock.get_requests(_AZURE_BLOB_PATH_RE)
    return [r for r in captured if r["method"] == "PUT"]


def _isolate_wiremock(wiremock) -> None:
    """Clear the auto-loaded baseline, then add only login + teardown mappings.

    WireMock's ``--root-dir`` auto-loads EVERY file under ``mappings/`` at
    startup, and the ``wiremock`` fixture's ``reset()`` restores that baseline.
    That includes parameterized files whose ``{{...}}`` placeholders are only
    substituted when loaded via ``add_mapping`` — the literal-placeholder copy
    competes at equal priority with our substituted copy and can
    non-deterministically win the match (e.g. serving ``endPoint`` literally as
    ``{{WIREMOCK_HTTP_URL}}`` (``_WIREMOCK_URL_PLACEHOLDER``)). Clearing all stubs first guarantees only our
    substituted mappings serve requests for this test. The next test's fixture
    ``reset()`` restores the file baseline, so this is scoped to our test.

    NOTE: delete-ALL (``DELETE /__admin/mappings``) is rejected with HTTP 500
    (``NotWritableException``) when the baseline contains read-only
    multi-mapping files, which it does. Per-id delete
    (``DELETE /__admin/mappings/{id}``) works on those same stubs, so we
    enumerate and delete individually.
    """
    base = wiremock.http_url()
    listing = requests.get(f"{base}/__admin/mappings", timeout=5).json()
    for mapping in listing.get("mappings", []):
        mapping_id = mapping.get("id") or mapping.get("uuid")
        if mapping_id:
            resp = requests.delete(f"{base}/__admin/mappings/{mapping_id}", timeout=5)
            assert resp.status_code in (200, 201), (
                f"failed to delete wiremock mapping {mapping_id}: {resp.status_code} {resp.text}"
            )
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")
    wiremock.add_mapping("session/logout_success.json")


# =============================================================================
# Happy path — upload (recovered 403 → debug only, NO warn)
# =============================================================================


def test_should_refresh_sas_and_succeed_when_azure_put_returns_403_on_the_first_attempt(
    int_test_connection_factory, wiremock, caplog, old_driver_azure_routes_to_wiremock
):
    """Scenario: should refresh SAS and succeed when Azure PUT returns 403 on the first attempt.

    Pins the any-403 trigger (fires refresh) and the recovered-403 logging
    floor: a RECOVERED 403
    logs at debug, never warn — warning on the normal SAS-expiry-then-recover
    path would cry wolf on every long transfer.

    The wiremock mapping returns reason ``"Server failed to authenticate the request."``
    which is in legacy Python's ``TOKEN_EXPIRATION_ERR_MESSAGE`` match-set, so both
    drivers refresh. ``old_driver_azure_routes_to_wiremock`` strips the legacy
    connector's mangled ``<account>.blob.<scheme>://`` host so its blob PUT actually
    reaches wiremock.
    """
    # Given Snowflake client is logged in to an Azure-backed deployment
    _isolate_wiremock(wiremock)
    # And Stage SAS is configured to return HTTP 403 on the first PUT attempt
    wiremock.add_mapping(
        "put_get/azure_sas_refresh_on_403_put_recover.json",
        placeholders={_WIREMOCK_URL_PLACEHOLDER: wiremock.http_url()},
    )

    with caplog.at_level(logging.WARNING, logger=CORE_LOGGER):
        with int_test_connection_factory(server_url=wiremock.http_url()) as connection, connection.cursor() as cursor:
            # When File is uploaded using PUT command
            cursor.execute(_put_command())
            put_results = cursor.fetchall()

    # Then The PUT query is re-issued to obtain a fresh stage credential
    azure_puts = _azure_puts(wiremock)
    assert any(_EXPIRED_SIG in r["url"] for r in azure_puts), (
        f"expected the first Azure PUT to carry the expired SAS (got urls: {[r['url'] for r in azure_puts]})"
    )
    assert any(_FRESH_SIG in r["url"] for r in azure_puts), (
        f"expected a retried Azure PUT carrying the REFRESHED SAS (got urls: {[r['url'] for r in azure_puts]})"
    )

    # And File should be uploaded successfully with the refreshed SAS
    _assert_all_uploaded(put_results)

    # And No warn-level log line is emitted for the recovered 403
    recovered_warns = [
        r for r in caplog.records if r.name == CORE_LOGGER and r.levelno >= logging.WARNING and "403" in r.message
    ]
    assert not recovered_warns, (
        f"recovered 403 must log at debug, NOT warn (severity tracks outcome). "
        f"Got warn-or-higher records: {[r.message for r in recovered_warns]}"
    )
    # And The request body is rebuilt for the post-refresh attempt
    # (upload is uncompressed + unencrypted, so the retried PUT's body must be the
    # raw source bytes). bodyAsBase64 is wiremock's lossless record of exactly what
    # it received (the `body` string is charset-decoded and can be lossy), so decode
    # it and compare byte-for-byte, mirroring the core test.
    source_bytes = repo_root().joinpath(*_TEST_DATA_REL).read_bytes()
    post_refresh_put = next((r for r in azure_puts if _FRESH_SIG in r["url"]), None)
    assert post_refresh_put is not None, (
        f"expected a post-refresh Azure PUT carrying the fresh SAS; urls: {[r['url'] for r in azure_puts]}"
    )
    b64 = post_refresh_put.get("bodyAsBase64")
    if b64:
        sent_bytes = base64.b64decode(b64)
    elif post_refresh_put.get("body"):
        # Fallback only if wiremock omitted the base64 field.
        sent_bytes = post_refresh_put["body"].encode()
    else:
        raise AssertionError(
            f"wiremock did not journal the PUT body for {post_refresh_put['url']!r}; "
            "cannot verify the rebuilt body (mock/journal gap, not a driver result)"
        )
    assert sent_bytes == source_bytes, (
        f"post-refresh PUT body must equal the uploaded source file byte-for-byte "
        f"({len(source_bytes)} bytes); got {len(sent_bytes)} bytes"
    )


# =============================================================================
# The reason phrase is irrelevant — UD's trigger is status-only (any 403).
# Legacy Python only refreshes when the reason matches specific token-expiry
# strings; UD refreshes regardless. This test uses a SYNTHETIC reason phrase
# to demonstrate the divergence (see docstring).
# =============================================================================


def test_should_recover_when_azure_put_returns_403_regardless_of_the_reason_phrase(
    int_test_connection_factory, wiremock, caplog, old_driver_azure_routes_to_wiremock
):
    """Scenario: should recover when Azure PUT returns 403 regardless of the reason phrase.

    Demonstrates the refresh TRIGGER mechanism: UD refreshes on any 403
    (status-only, HTTP/2-safe) and recovers; legacy Python only refreshes
    when the 403 reason matches one of two hard-coded ``TOKEN_EXPIRATION_ERR_MESSAGE``
    strings, so it does not refresh here and the PUT fails.

    SYNTHETIC REASON — read before trusting this as a real-world repro. The first
    (expiredsig) Azure PUT returns 403 with reason "Signature fields not well
    formed.", a HAND-AUTHORED phrase (not captured from real Azure) chosen
    specifically to sit OUTSIDE legacy Python's match-set ("Signature not valid in
    the specified time frame", "Server failed to authenticate the request."). The
    captured REAL Azure SAS-expiry reason is "Server failed to authenticate the
    request.", which DOES match Python's gate — so over HTTP/1.1 with a real
    reason phrase the two drivers would NOT diverge on a true expiry. The recorded
    divergence is the trigger MECHANISM (legacy reason-gated vs UD status-only): it
    widens for any 403 whose reason legacy does not list. This test isolates that
    mechanism with a synthetic non-matching reason as the wire-level proxy.

    ``old_driver_azure_routes_to_wiremock`` strips the legacy connector's
    mangled ``<account>.blob.<scheme>://`` host so its Azure PUT actually reaches
    wiremock and receives the real 403 (the new driver honours the
    scheme-prefixed ``endPoint`` natively and needs no routing).
    """
    # Given Snowflake client is logged in to an Azure-backed deployment
    _isolate_wiremock(wiremock)
    # And Stage SAS is configured to return HTTP 403 with reason "<reason>"
    mapping = "put_get/azure_sas_refresh_on_403_recover_reason_mismatch.json"
    # And the refresh returns a SAS that Azure accepts
    wiremock.add_mapping(mapping, placeholders={_WIREMOCK_URL_PLACEHOLDER: wiremock.http_url()})

    # When File is uploaded using PUT command
    put_results = None
    upload_error = None
    # Outcome diverges by driver (UD recovers, legacy raises), so capture both
    # uniformly here and let the driver-gated assertions below discriminate.
    try:
        with int_test_connection_factory(server_url=wiremock.http_url()) as connection, connection.cursor() as cursor:
            cursor.execute(_put_command())
            put_results = cursor.fetchall()
    except Error as e:
        upload_error = e

    # Then The PUT query is re-issued to obtain a fresh stage credential
    azure_puts = _azure_puts(wiremock)
    assert any(_EXPIRED_SIG in r["url"] for r in azure_puts), (
        f"expected the first Azure PUT to carry the expired SAS (got urls: {[r['url'] for r in azure_puts]})"
    )

    if NEW_DRIVER_ONLY("BD#40"):
        # UD refreshes on ANY 403 (status-only, HTTP/2-safe): a freshsig PUT is
        # retried and the upload succeeds.
        assert upload_error is None, f"new driver must recover via refresh, not raise; got: {upload_error!r}"
        # And The refreshed SAS is accepted
        assert any(_FRESH_SIG in r["url"] for r in azure_puts), (
            f"new driver must refresh on ANY 403 and retry with the fresh SAS "
            f"(got urls: {[r['url'] for r in azure_puts]})"
        )
        # And The upload succeeds
        _assert_all_uploaded(put_results)

    if OLD_DRIVER_ONLY("BD#40"):
        # Legacy Python's reason-gate does NOT match this 403 reason, so it never
        # refreshes: no freshsig PUT, and the operation surfaces the 403 instead.
        assert upload_error is not None, "legacy reason-gate must NOT refresh on this 403 reason; PUT must fail"
        err_text = str(upload_error).lower()
        assert "403" in err_text or "forbidden" in err_text, (
            f"legacy PUT must surface the underlying 403; got: {upload_error!r}"
        )
        assert not any(_FRESH_SIG in r["url"] for r in azure_puts), (
            f"legacy reason-gate must NOT refresh on this 403 reason — no freshsig PUT expected; "
            f"urls: {[r['url'] for r in azure_puts]}"
        )


# =============================================================================
# Refresh failure path (refresh mechanism itself fails → error-level log)
# =============================================================================


def test_should_surface_terminal_error_when_put_sas_refresh_itself_fails(int_test_connection_factory, wiremock, caplog):
    """Scenario: should surface terminal error when PUT SAS refresh itself fails.

    When GS errors on the re-issued PUT query, the operation surfaces a terminal
    error naming SAS refresh as the cause: a refresh-mechanism failure is terminal
    and logs at ERROR.

    UD-ONLY: this pins the UD's BOUNDED handling of a failed refresh. The mapping's
    blob PUT returns reason ``"Server failed to authenticate the request."`` (in
    legacy's ``TOKEN_EXPIRATION_ERR_MESSAGE`` set), so legacy also enters its refresh
    path — but the refresh re-issue returns HTTP 500, and legacy's query RetryCtx has
    ``network_timeout=None`` (infinite by default) with no attempt cap, so
    ``should_retry`` stays True and legacy retries the 500 forever with exponential
    backoff, hanging the transfer until pytest-timeout fires. The UD bounds the
    refresh failure and raises a terminal error instead. There is no honest legacy
    assertion for "bounded terminal error on refresh failure", so the whole scenario
    is gated to the UD.
    """
    if OLD_DRIVER_ONLY("BD#41"):
        pytest.skip(
            "UD-only: legacy retries the refresh's GS 500 indefinitely (network_timeout "
            "infinite by default), so it cannot surface a bounded terminal refresh error"
        )
    # Given Snowflake client is logged in to an Azure-backed deployment
    _isolate_wiremock(wiremock)
    # And Stage SAS is configured to return HTTP 403 on the PUT
    refresh_fails_mapping = "put_get/azure_sas_refresh_on_403_refresh_fails.json"
    # And Snowflake GS is unreachable for the refresh query
    wiremock.add_mapping(refresh_fails_mapping, placeholders={_WIREMOCK_URL_PLACEHOLDER: wiremock.http_url()})

    with caplog.at_level(logging.ERROR, logger=CORE_LOGGER):
        # pytest.raises wraps the WHOLE connection context: the terminal error must
        # propagate OUT of the `with connection` body so __exit__ takes the
        # rollback path (_connection.py:368). If we caught it inside, __exit__
        # would see a clean exit, commit(), and re-drive the failed upload uncaught.
        # When File is uploaded using PUT command
        with pytest.raises(Error) as excinfo:
            with (
                int_test_connection_factory(server_url=wiremock.http_url()) as connection,
                connection.cursor() as cursor,
            ):
                cursor.execute(_put_command())

    # Then The PUT query is re-issued to obtain a fresh stage credential
    gs_queries = wiremock.get_requests(_GS_QUERY_PATH_RE)
    assert len(gs_queries) >= 2, (
        f"expected the initial PUT-command query PLUS at least one refresh re-issue; "
        f"got {len(gs_queries)} GS query-requests"
    )
    # And An error is raised indicating SAS refresh failed
    assert str(excinfo.value), f"a terminal error must be raised when SAS refresh fails; got: {excinfo.value!r}"
    # refresh failed, so the blob PUT must NOT be retried — exactly the original PUT reached the wire
    azure_puts = _azure_puts(wiremock)
    assert len(azure_puts) == 1, (
        f"refresh failed, so no retry PUT should be issued — exactly the original PUT; "
        f"got {len(azure_puts)}: {[r['url'] for r in azure_puts]}"
    )
    # And An error-level log line is emitted naming the refresh-failure reason
    refresh_errors = [
        r
        for r in caplog.records
        if r.name == CORE_LOGGER and r.levelno >= logging.ERROR and "refresh" in r.message.lower()
    ]
    assert refresh_errors, (
        f"expected a Core error-log line naming the refresh failure (refresh-mechanism failure is "
        f"terminal -> error). Got core-error records: "
        f"{[r.message for r in caplog.records if r.name == CORE_LOGGER and r.levelno >= logging.ERROR]}"
    )


# =============================================================================
# Non-token 403 (any-403 trigger; terminal → warn-level log with structured fields)
# =============================================================================


def test_should_retry_then_fail_when_azure_put_403_is_not_caused_by_sas_expiry(
    int_test_connection_factory, wiremock, caplog, old_driver_azure_routes_to_wiremock
):
    """Scenario: should retry then fail when Azure PUT 403 is not caused by SAS expiry.

    Any 403 triggers refresh in the UD; if the new SAS still gets 403 (bucket-policy
    denial, misconfiguration) the operation surfaces the terminal 403. Because
    this 403 is NOT a known token-expiry reason, the terminal failure logs at
    WARN (the contract-drift signal) with status, the Azure ``<Code>``,
    and a SAS-redacted URL.

    The wiremock mapping uses reason ``"This request is not authorized to perform this
    operation."`` which is NOT in legacy Python's ``TOKEN_EXPIRATION_ERR_MESSAGE``
    match-set. Legacy therefore does NOT refresh: it fails terminally on the first
    PUT without issuing a second GS query or a freshsig PUT. The UD-specific
    assertions (freshsig retry, WARN log with ``AuthorizationFailure``, SAS redaction)
    are gated with ``NEW_DRIVER_ONLY``. ``old_driver_azure_routes_to_wiremock`` strips
    the legacy connector's mangled ``<account>.blob.<scheme>://`` host so the initial
    blob PUT reaches wiremock for both drivers.
    """
    # Given Snowflake client is logged in to an Azure-backed deployment
    _isolate_wiremock(wiremock)
    # And Stage SAS is configured to return HTTP 403 for a non-token reason
    wiremock.add_mapping(
        "put_get/azure_sas_refresh_on_403_nontoken.json",
        placeholders={_WIREMOCK_URL_PLACEHOLDER: wiremock.http_url()},
    )

    with caplog.at_level(logging.WARNING, logger=CORE_LOGGER):
        # pytest.raises wraps the whole connection context so the terminal 403
        # propagates out (taking __exit__'s rollback path) rather than being
        # caught inside and re-driven by commit-on-clean-exit.
        # When File is uploaded using PUT command
        with pytest.raises(Error, match="403") as excinfo:
            with (
                int_test_connection_factory(server_url=wiremock.http_url()) as connection,
                connection.cursor() as cursor,
            ):
                cursor.execute(_put_command())

    azure_puts = _azure_puts(wiremock)
    # Then The initial PUT carried the expired SAS (both drivers)
    assert any(_EXPIRED_SIG in r["url"] for r in azure_puts), (
        f"expected the first Azure PUT to carry the expired SAS; urls: {[r['url'] for r in azure_puts]}"
    )
    # And An error is raised indicating Azure storage returned HTTP 403 (both drivers)
    err_text = str(excinfo.value).lower()
    assert "403" in err_text or "forbidden" in err_text, (
        f"terminal error must surface the underlying 403; got: {excinfo.value!r}"
    )

    if NEW_DRIVER_ONLY("BD#40"):
        # UD refreshes on ANY 403 (status-only, HTTP/2-safe): a freshsig PUT is
        # retried and also rejected → terminal WARN with structured fields.
        # And The refreshed SAS is also rejected with HTTP 403
        assert any(_FRESH_SIG in r["url"] for r in azure_puts), (
            f"UD must retry with the REFRESHED SAS (also 403); urls: {[r['url'] for r in azure_puts]}"
        )
        # And A warn-level log line is emitted at status 403
        refresh_warns = [
            r
            for r in caplog.records
            if r.name == CORE_LOGGER
            and r.levelno >= logging.WARNING
            and "403" in r.message
            and "azure" in r.message.lower()
        ]
        assert refresh_warns, (
            f"expected a warn line for the terminal non-token 403; got "
            f"{[r.message for r in caplog.records if r.name == CORE_LOGGER and r.levelno >= logging.WARNING]}"
        )
        warn_msg = refresh_warns[0].message
        # And The warn log names the Azure error code
        assert "AuthorizationFailure" in warn_msg, (
            f"warn line must name the Azure error code AuthorizationFailure; got: {warn_msg!r}"
        )
        # And The warn log carries a SAS-redacted URL
        for r in refresh_warns:
            assert _EXPIRED_SIG not in r.message and _FRESH_SIG not in r.message, (
                f"SAS signature must be redacted in the warn-log; got: {r.message!r}"
            )

    if OLD_DRIVER_ONLY("BD#40"):
        # Legacy reason-gate does NOT match this 403 reason, so it never refreshes:
        # no freshsig PUT is issued and the operation fails on the first attempt.
        assert not any(_FRESH_SIG in r["url"] for r in azure_puts), (
            f"legacy must NOT refresh on this non-token 403 reason — no freshsig PUT expected; "
            f"urls: {[r['url'] for r in azure_puts]}"
        )
