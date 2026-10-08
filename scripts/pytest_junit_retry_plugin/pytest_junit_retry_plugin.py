"""pytest plugin: record reruns and patch --junitxml after it is written."""

from __future__ import annotations

from pathlib import Path

import pytest

from annotate_pytest_junit_retries import Attempt, annotate


_FAILED = frozenset({"failed", "rerun", "error"})
_attempts: dict[str, list[Attempt]] = {}


def record_attempts(nodeid: str, outcome: str, message: str = "") -> None:
    if outcome not in _FAILED and outcome != "passed":
        return
    _attempts.setdefault(nodeid, []).append(Attempt(outcome, message))


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    message = ""
    if report.longrepr:
        message = str(report.longrepr)[:2000]
    if report.when == "call":
        record_attempts(report.nodeid, report.outcome, message)
    elif report.outcome in _FAILED:
        record_attempts(report.nodeid, report.outcome, message)


@pytest.hookimpl(trylast=True)
def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    _ = exitstatus
    if getattr(session.config, "workerinput", None) is not None:
        return
    xmlpath = getattr(session.config.option, "xmlpath", None)
    if not xmlpath:
        return
    path = Path(xmlpath)
    if not path.is_file():
        return
    if not any(len(history) >= 2 for history in _attempts.values()):
        return
    path.write_text(
        annotate(path.read_text(encoding="utf-8"), _attempts), encoding="utf-8"
    )
