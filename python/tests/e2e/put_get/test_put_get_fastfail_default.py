"""
E2E tests locking in Python's default PUT/GET fastfail behavior.

Python's `WrapperPresets.put_get_fastfail_default` is `true`: the first
per-file transfer error aborts the whole batch and the driver raises. ODBC's
default is `false` (collect-all): every file is still attempted, but PUT then
raises an aggregate error listing all failures, while GET returns
`status="ERROR"` rows. These tests guard against that default ever flipping
for Python.
"""

import os
import sys
import tempfile

from pathlib import Path

import pytest

from snowflake.connector.errors import OperationalError
from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.e2e.put_get.put_get_helper import create_temporary_stage


def _can_run_permission_based_test() -> bool:
    """Permission-based failure injection only works for a non-root Unix
    caller: root bypasses owner-permission checks, and Windows doesn't honor
    POSIX mode bits the way `os.chmod` expects — same constraint as the ODBC
    e2e fastfail tests (`can_run_permission_based_test` in
    put_get_fastfail_default.cpp)."""
    if sys.platform == "win32":
        return False
    return os.geteuid() != 0


@pytest.mark.skipif(
    not _can_run_permission_based_test(),
    reason="Permission-based failure injection requires a non-root Unix user",
)
def test_should_raise_instead_of_returning_error_rows_when_put_batch_has_a_failing_file(connection):
    """Regression guard for SNOW-3838438: Python's `put_get_fastfail_default`
    must stay `true` so PUT raises on the first per-file error rather than
    attempting every file and returning collect-all ERROR rows.

    `SOURCE_COMPRESSION=GZIP` disables the reference connector's content-sniffing
    auto_detect pre-scan (file_transfer_agent.py's `_process_file_compression_type`),
    which otherwise does a bare, unwrapped `open()` on every file before transfer
    even starts — without it, a chmod'd-unreadable file raises a raw
    `PermissionError` there instead of the wrapped error this test wants to guard.
    With auto_detect off, the failure happens during transfer instead, where both
    drivers wrap it as `OperationalError` (SNOW-3838438 follow-up: the universal
    driver previously mapped local transfer I/O failures to
    `ErrorKind::InternalError` (then treated as a query-failure bucket), not
    transfer faults; fixed to use the dedicated `ErrorKind::Io` ->
    `OperationalError`, matching the reference connector's
    own classification). The reference connector also, unlike the universal
    driver, transfers in parallel and attempts every file regardless of
    others' failures — its own collect-all-style default."""
    with tempfile.TemporaryDirectory() as temp_dir, connection.cursor() as cursor:
        temp_path = Path(temp_dir)

        # Given four files, glob-sorted as 1_ok < 2_blocked < 3_blocked < 4_ok:
        # the first uploads before any failure; the second and third are both
        # unreadable (chmod 0, same injection technique as the ODBC e2e
        # fastfail tests) so a regression to collect-all would fail both of
        # them, not just one; the fourth would also upload cleanly but sorts
        # after every failure.
        ok_file1 = temp_path / "fastfail_1_ok.csv"
        ok_file1.write_text("1,2,3\n")

        blocked_file1 = temp_path / "fastfail_2_blocked.csv"
        blocked_file1.write_text("4,5,6\n")
        blocked_file1.chmod(0o000)

        blocked_file2 = temp_path / "fastfail_3_blocked.csv"
        blocked_file2.write_text("7,8,9\n")
        blocked_file2.chmod(0o000)

        ok_file2 = temp_path / "fastfail_4_ok.csv"
        ok_file2.write_text("10,11,12\n")

        stage_name = create_temporary_stage(cursor, "TEST_STAGE_PUT_FASTFAIL")
        wildcard_pattern = (temp_path / "fastfail_*.csv").as_posix()

        # When the batch is PUT under Python's default settings (no PUT_FASTFAIL
        # override); SOURCE_COMPRESSION=GZIP keeps the failure inside the wrapped
        # transfer path on both drivers (see docstring)
        put_command = f"PUT 'file://{wildcard_pattern}' @{stage_name} SOURCE_COMPRESSION=GZIP"

        # Then the whole batch raises on the first failure rather than
        # returning a result set with UPLOADED and ERROR rows. Both drivers
        # wrap this as OperationalError -- a transfer/environmental fault,
        # not a query/programming error.
        with pytest.raises(OperationalError) as excinfo:
            cursor.execute(put_command)
        error = excinfo.value

        cursor.execute(f"LIST @{stage_name}")
        listed_names = [row[0] for row in cursor.fetchall()]

        if IS_UNIVERSAL_DRIVER:
            assert "Failed to upload files" in error.msg

            # And (ODBC-parity check, mirroring HasDiagMessage("blocked1.csv")
            # && HasDiagMessage("blocked2.csv") in the ODBC collect-all test)
            # the error reflects exactly one failure, not two: collect-all's
            # aggregate is "PUT failed for {failure_count} file(s):\n{failures}",
            # so with two unreadable files a regression to collect-all would
            # say "PUT failed for 2 file(s)" here instead of the bare per-file
            # error.
            assert "PUT failed for" not in error.msg

            # No "names exactly one blocked file" check on this driver:
            # FileManagerError::Io (sf_core/src/file_manager/mod.rs) has no
            # filename field, so the universal driver's per-file error never
            # names the file at all. (The reference connector's message
            # can't name it either, for a different reason -- see the else
            # branch below.)
        else:
            # The reference connector transfers in parallel and attempts
            # every file regardless of others' failures, so both ok files
            # succeed despite the batch ultimately raising.
            assert len(listed_names) == 2
            assert any("fastfail_1_ok.csv" in name for name in listed_names)
            assert any("fastfail_4_ok.csv" in name for name in listed_names)

            # No "names exactly one blocked file" check here either (same
            # gap as the universal-driver branch, for a different reason):
            # result() (file_transfer_agent.py) builds the message from
            # repr(meta.error_details), and repr() of a plain OSError/
            # PermissionError does NOT include the filename -- only str()
            # does (verified: repr(PermissionError(13, "Permission denied"))
            # omits the path that str() shows). So this message can never
            # name either blocked file, on any driver.
