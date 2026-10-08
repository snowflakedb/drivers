"""Inject Jenkins flakyFailure / rerunFailure into pytest JUnit XML.

pytest-rerunfailures plus the stock junitxml plugin keep only the last
attempt. Call-phase reports still see every rerun, so we reconstruct retry
history and write Surefire-style children after the XML is written.
"""

from __future__ import annotations

from typing import NamedTuple
from xml.etree import ElementTree as ET


_FAILED = frozenset({"failed", "rerun", "error"})


class Attempt(NamedTuple):
    outcome: str
    message: str


def keys_for_nodeid(nodeid: str) -> set[str]:
    """The classname::name key pytest junitxml emits for a nodeid.

    Mirrors pytest's own `mangle_test_address`: split off any parametrize
    suffix before splitting on "::" so it stays attached to the test name,
    and keep every intermediate class name in the dotted classname.
    """
    path, bracket, params = nodeid.partition("[")
    names = path.split("::")
    try:
        names.remove("()")
    except ValueError:
        pass
    if len(names) < 2:
        return set()
    file_part = names[0].replace("\\", "/")
    dotted = file_part.replace("/", ".")
    if dotted.endswith(".py"):
        dotted = dotted[:-3]
    names[0] = dotted
    names[-1] = names[-1] + bracket + params
    classname = ".".join(names[:-1])
    name = names[-1]
    return {f"{classname}::{name}"}


def annotate(junit_xml: str, attempts: dict[str, list[Attempt]]) -> str:
    by_key: dict[str, list[Attempt]] = {}
    for nodeid, history in attempts.items():
        for key in keys_for_nodeid(nodeid):
            by_key[key] = history

    declaration, body = "", junit_xml
    if junit_xml.lstrip().startswith("<?xml"):
        head, _, rest = junit_xml.partition("?>")
        declaration = head + "?>"
        body = rest

    root = ET.fromstring(body)
    cases = root.findall(".//testcase")
    if root.tag == "testcase":
        cases = [root]

    parents = {id(child): parent for parent in root.iter() for child in parent}
    cases_by_key: dict[str, list[ET.Element]] = {}
    for case in cases:
        classname = case.get("classname") or ""
        name = case.get("name") or ""
        cases_by_key.setdefault(f"{classname}::{name}", []).append(case)

    for key, duplicates in cases_by_key.items():
        history = by_key.get(key)
        if not history or len(history) < 2:
            continue
        # A rerun attempt still finalizes its own empty testcase on teardown,
        # so pytest emits one XML element per attempt under the same key.
        # Only the last one gets the reconstructed history attached.
        case = duplicates[-1]
        for stale in duplicates[:-1]:
            parent = parents.get(id(stale))
            if parent is not None:
                parent.remove(stale)
        if (
            case.find("flakyFailure") is not None
            or case.find("rerunFailure") is not None
        ):
            continue
        last = history[-1]
        prior_failed = [h for h in history[:-1] if h.outcome in _FAILED]
        if not prior_failed:
            continue
        child_tag = "flakyFailure" if last.outcome == "passed" else "rerunFailure"
        for i, attempt in enumerate(prior_failed, start=1):
            child = ET.Element(
                child_tag,
                {
                    "type": "pytest.rerunfailures",
                    "message": attempt.message or f"pytest attempt {i} failed",
                },
            )
            if attempt.message:
                child.text = attempt.message
            case.append(child)
    xml = ET.tostring(root, encoding="unicode")
    if declaration:
        return declaration + xml
    return xml
