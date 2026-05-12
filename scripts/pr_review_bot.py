#!/usr/bin/env python3
"""PR review automation bot.

This script is the orchestration brain behind ``.github/workflows/pr-review-bot.yml``.
It does no HTTP itself — GitHub calls are delegated to the pre-installed
``gh`` CLI (via subprocess) and Slack calls are delegated to
``slackapi/slack-github-action`` in the workflow. The Python here only
classifies state, picks a reviewer, formats messages, and writes a Slack
payload file that the Slack action posts in the next step.

Subcommands
-----------
``assign``
    Pick a random reviewer from the candidates listed in
    ``.github/CODEOWNERS`` (or another path via ``CODEOWNERS_PATH``),
    excluding the PR author and any user already requested for review.
    Calls ``gh pr edit --add-reviewer X --add-assignee X``. Then writes a
    Slack payload describing the assignment to ``$SLACK_PAYLOAD_FILE`` so
    the next workflow step can post it via
    ``slackapi/slack-github-action``.

    Designed to be run from a ``pull_request_target`` workflow on the
    ``opened`` and ``ready_for_review`` activity types. Drafts are skipped.

    The candidate pool is the union of all individual ``@login`` entries in
    CODEOWNERS — team references (``@org/team-slug``) are ignored on
    purpose so the bot does not need ``read:org`` to resolve membership.

``remind``
    Iterate every open non-draft PR in the repository (via ``gh``) and
    write a digest Slack payload listing PRs that are *waiting on a
    reviewer's action* — i.e. no review with state ``APPROVED`` or
    ``CHANGES_REQUESTED``. PRs where a requested reviewer has only
    ``COMMENTED`` are flagged with a note that comments do not count as a
    review. Each entry includes the time elapsed since the *initial*
    ``review_requested`` event.

Required environment variables
------------------------------
``GH_TOKEN`` (or ``GITHUB_TOKEN``)
    Token used by ``gh`` to read PRs, request reviewers, add assignees,
    and read commit metadata. A workflow with ``pull-requests: write``
    and ``issues: write`` permissions is sufficient.

``GH_REPO``
    ``owner/name`` of the repository. Also consumed by the ``gh`` CLI.

``SLACK_CHANNEL``
    Channel name (e.g. ``drivers-review``) or channel ID written into
    the generated Slack payload's ``channel`` field.

``SLACK_PAYLOAD_FILE``
    Path where the generated ``chat.postMessage`` JSON payload should be
    written. The Slack action reads it via ``payload-file-path``. If the
    script decides there is nothing to post, the file is not created and
    the step output ``skip=true`` is set.

Assign-only environment variables
---------------------------------
``PR_NUMBER``
    Pull request number to operate on.

``CODEOWNERS_PATH`` (optional)
    Path to the CODEOWNERS file to parse. Defaults to
    ``.github/CODEOWNERS``.

Reviewer display names
----------------------
The Slack message tags each reviewer as ``@<handle>`` where ``<handle>``
is the git ``commit.author.name`` of their most recent commit in this
repo, lowercased and dot-separated to match the typical corporate Slack
handle convention (e.g. ``Maxymilian Kowalski`` from
``Author: Maxymilian Kowalski <…@snowflake.com>`` becomes
``@maxymilian.kowalski``). Unicode characters are folded to ASCII so
diacritics don't break the handle. When the user has no commits in the
repo, the message falls back to ``@<github-login>``. The
``chat.postMessage`` payload sets ``link_names: true`` so Slack
auto-converts any ``@handle`` that matches a workspace member to a real
mention; otherwise the prefixed handle renders as plain text.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import random
import re
import subprocess
import sys
import unicodedata
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
log = logging.getLogger("pr-review-bot")

DEFAULT_CODEOWNERS_PATH = Path(".github/CODEOWNERS")

# Review states that mean the reviewer has taken action on the PR.
ACTIONED_STATES = {"APPROVED", "CHANGES_REQUESTED"}


# ---------------------------------------------------------------------------
# gh CLI wrappers
# ---------------------------------------------------------------------------


def _gh(*args: str, check: bool = True) -> str:
    """Invoke ``gh`` with *args* and return stdout.

    Authentication is taken from the standard ``GH_TOKEN`` / ``GITHUB_TOKEN``
    environment variables, the same as any other ``gh`` invocation.
    """
    cmd = ["gh", *args]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if check and result.returncode != 0:
        raise RuntimeError(
            f"`gh {' '.join(args)}` failed (exit {result.returncode}): "
            f"{result.stderr.strip()}"
        )
    return result.stdout


def gh_api_json(path: str, *, paginate: bool = False) -> Any:
    """Run ``gh api <path>`` and return the parsed JSON output.

    ``--paginate`` makes ``gh`` walk the ``Link`` header and concatenate
    JSON arrays into a single array — exactly what we want for the list
    endpoints we hit (PRs, reviews, timeline, commits).
    """
    args = ["api"]
    if paginate:
        args.append("--paginate")
    args.append(path)
    out = _gh(*args).strip()
    if not out:
        return None
    return json.loads(out)


def gh_pr_assign(repo: str, pr_number: int, login: str) -> None:
    """``gh pr edit`` to add *login* as both reviewer and assignee."""
    _gh(
        "pr",
        "edit",
        str(pr_number),
        "--repo",
        repo,
        "--add-reviewer",
        login,
        "--add-assignee",
        login,
    )


# ---------------------------------------------------------------------------
# GitHub data lookups (all via gh CLI)
# ---------------------------------------------------------------------------


def get_pr(repo: str, pr_number: int) -> dict:
    data = gh_api_json(f"/repos/{repo}/pulls/{pr_number}")
    if not isinstance(data, dict):
        raise RuntimeError(f"Unexpected PR payload for #{pr_number}")
    return data


def list_open_prs(repo: str) -> list[dict]:
    return (
        gh_api_json(
            f"/repos/{repo}/pulls?state=open&sort=created&direction=asc&per_page=100",
            paginate=True,
        )
        or []
    )


def list_pr_reviews(repo: str, pr_number: int) -> list[dict]:
    return (
        gh_api_json(
            f"/repos/{repo}/pulls/{pr_number}/reviews?per_page=100", paginate=True
        )
        or []
    )


def first_review_request_time(repo: str, pr_number: int) -> datetime | None:
    """Earliest ``review_requested`` event timestamp on a PR, or ``None``.

    Removals (``review_request_removed``) are intentionally ignored so the
    timestamp reflects the very first time anyone was put on the hook.
    """
    events = (
        gh_api_json(
            f"/repos/{repo}/issues/{pr_number}/timeline?per_page=100",
            paginate=True,
        )
        or []
    )
    earliest: datetime | None = None
    for ev in events:
        if ev.get("event") != "review_requested":
            continue
        created = ev.get("created_at")
        if not created:
            continue
        try:
            ts = datetime.strptime(created, "%Y-%m-%dT%H:%M:%SZ").replace(
                tzinfo=timezone.utc
            )
        except ValueError:
            continue
        if earliest is None or ts < earliest:
            earliest = ts
    return earliest


def github_commit_author_name(repo: str, login: str) -> str | None:
    """Return the git ``commit.author.name`` of the user's most recent
    commit in *repo*, or ``None`` if they have no commits there.

    This is the "Name" portion of ``Author: Maxymilian Kowalski
    <…@snowflake.com>``; the email is deliberately not used.
    """
    try:
        data = gh_api_json(
            f"/repos/{repo}/commits?author={login}&per_page=1"
        )
    except RuntimeError:
        return None
    if not isinstance(data, list) or not data:
        return None
    commit = (data[0] or {}).get("commit") or {}
    author = commit.get("author") or {}
    name = (author.get("name") or "").strip()
    return name or None


# ---------------------------------------------------------------------------
# CODEOWNERS parsing
# ---------------------------------------------------------------------------


def parse_codeowners(path: Path = DEFAULT_CODEOWNERS_PATH) -> list[str]:
    """Return the unique individual ``@login`` owners listed in *path*.

    Team references (``@org/team-slug``) and email addresses are skipped.
    """
    if not path.exists():
        raise FileNotFoundError(f"CODEOWNERS file not found at {path}")

    individuals: set[str] = set()
    skipped_teams: set[str] = set()

    for raw_line in path.read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        tokens = line.split()
        if len(tokens) < 2:
            continue
        for token in tokens[1:]:
            if not token.startswith("@"):
                continue
            handle = token[1:]
            if "/" in handle:
                skipped_teams.add(token)
                continue
            individuals.add(handle)

    if skipped_teams:
        # The bot intentionally does not resolve team membership (no
        # read:org needed). Teams are commonly kept alongside individuals
        # for native CODEOWNERS purposes, so this is INFO, not a warning.
        log.info(
            "Skipped team reference(s) in %s: %s (individuals are used "
            "for random reviewer selection).",
            path,
            ", ".join(sorted(skipped_teams)),
        )

    return sorted(individuals)


# ---------------------------------------------------------------------------
# Reviewer display-name lookup
# ---------------------------------------------------------------------------


# Characters that don't decompose under NFKD but still need an ASCII
# equivalent for handle generation (Polish ł, Nordic ø, German ß, etc.).
_HANDLE_ASCII_FOLD = str.maketrans(
    {
        "Ł": "L",
        "ł": "l",
        "Đ": "D",
        "đ": "d",
        "Ø": "O",
        "ø": "o",
        "Æ": "Ae",
        "æ": "ae",
        "Œ": "Oe",
        "œ": "oe",
        "Þ": "Th",
        "þ": "th",
        "ß": "ss",
    }
)


def name_to_handle(display: str) -> str:
    """Turn a real name like ``"Maxymilian Kowalski"`` into a Slack-style
    handle like ``"maxymilian.kowalski"``.

    Steps: fold a small set of Unicode letters that don't decompose
    cleanly (e.g. Polish ``ł``), NFKD-decompose the rest and drop
    non-ASCII, lowercase, drop characters Slack handles don't allow, and
    join the resulting tokens with a dot.
    """
    folded = display.translate(_HANDLE_ASCII_FOLD)
    ascii_form = (
        unicodedata.normalize("NFKD", folded)
        .encode("ascii", "ignore")
        .decode("ascii")
    )
    parts: list[str] = []
    for raw in ascii_form.lower().split():
        # Slack handles allow lowercase letters, digits, dot, hyphen, and
        # underscore. Strip everything else (apostrophes, punctuation, etc.).
        cleaned = re.sub(r"[^a-z0-9_-]", "", raw)
        if cleaned:
            parts.append(cleaned)
    return ".".join(parts)


class ReviewerDisplay:
    """Cache GitHub login -> ``@<handle>`` Slack mention string.

    Display name is taken from ``commit.author.name`` of the user's most
    recent commit in the repo (the ``Name`` portion of
    ``Author: Maxymilian Kowalski <…@snowflake.com>``), normalised to a
    dot-separated lowercase handle (``maxymilian.kowalski``), and
    prefixed with ``@`` so Slack's ``link_names`` feature can auto-convert
    it to a real mention when the handle matches a workspace member.

    If the user has no commits in the repo (or the name normalises to an
    empty handle), falls back to ``@<github-login>``.
    """

    def __init__(self, repo: str | None) -> None:
        self._repo = repo
        self._cache: dict[str, str] = {}

    def name(self, login: str) -> str:
        if not login:
            return ""
        if login in self._cache:
            return self._cache[login]

        display: str | None = None
        if self._repo:
            display = github_commit_author_name(self._repo, login)

        handle: str | None = None
        if display:
            handle = name_to_handle(display)

        if handle:
            mention = f"@{handle}"
            log.info(
                "Display handle for %s -> %r (from commit author %r)",
                login,
                mention,
                display,
            )
        else:
            mention = f"@{login}"
            log.info(
                "No usable commit author name for %s in %s; falling back to "
                "GitHub login.",
                login,
                self._repo,
            )

        self._cache[login] = mention
        return mention


# ---------------------------------------------------------------------------
# Workflow output / Slack payload helpers
# ---------------------------------------------------------------------------


def set_gh_output(name: str, value: str) -> None:
    """Append a ``key<<EOF...EOF`` block to the step's ``$GITHUB_OUTPUT``.

    No-op outside Actions (``$GITHUB_OUTPUT`` unset).
    """
    out = os.environ.get("GITHUB_OUTPUT")
    if not out:
        return
    delim = "EOF_PR_REVIEW_BOT"
    with open(out, "a") as f:
        f.write(f"{name}<<{delim}\n{value}\n{delim}\n")


def write_slack_payload(path: Path, channel: str, text: str) -> None:
    payload = {
        "channel": channel,
        "text": text,
        # Have Slack auto-resolve any "@handle" in the text into a real
        # <@U…> mention when the handle matches a workspace member.
        "link_names": True,
        "unfurl_links": False,
        "unfurl_media": False,
    }
    path.write_text(json.dumps(payload))


def _slack_runtime() -> tuple[str | None, Path | None]:
    """Return ``(channel, payload_file)`` from the environment."""
    channel = os.environ.get("SLACK_CHANNEL", "").strip() or None
    payload = os.environ.get("SLACK_PAYLOAD_FILE", "").strip() or None
    return (channel, Path(payload) if payload else None)


# ---------------------------------------------------------------------------
# Subcommand: assign
# ---------------------------------------------------------------------------


def _pick_reviewer(
    candidates: Iterable[str], excluded: Iterable[str]
) -> str | None:
    excluded_lower = {u.lower() for u in excluded}
    pool = [c for c in candidates if c.lower() not in excluded_lower]
    if not pool:
        return None
    return random.choice(pool)


def _skip_assign(reason: str) -> int:
    log.info("%s", reason)
    set_gh_output("skip", "true")
    return 0


def cmd_assign(args: argparse.Namespace) -> int:
    repo = os.environ["GH_REPO"]
    pr_number_env = os.environ.get("PR_NUMBER", "").strip()
    if not pr_number_env:
        log.error("PR_NUMBER is required for assign mode")
        return 2
    pr_number = int(pr_number_env)
    codeowners_path = Path(
        os.environ.get("CODEOWNERS_PATH") or DEFAULT_CODEOWNERS_PATH
    )
    channel, payload_file = _slack_runtime()

    pr = get_pr(repo, pr_number)
    if pr.get("draft"):
        return _skip_assign(f"PR #{pr_number} is a draft; skipping assignment.")
    if pr.get("state") != "open":
        return _skip_assign(
            f"PR #{pr_number} is not open (state={pr.get('state')}); skipping."
        )

    author = pr["user"]["login"]
    title = pr["title"]
    html_url = pr["html_url"]

    already_requested = [u["login"] for u in pr.get("requested_reviewers", []) or []]
    if already_requested:
        return _skip_assign(
            f"PR #{pr_number} already has reviewers requested "
            f"({', '.join(already_requested)}); skipping auto-assign."
        )

    log.info("Reading reviewer pool from %s ...", codeowners_path)
    try:
        candidates = parse_codeowners(codeowners_path)
    except FileNotFoundError as e:
        log.error("%s. Aborting assign.", e)
        return 1

    log.info("CODEOWNERS lists %d individual reviewer(s).", len(candidates))
    if not candidates:
        log.warning(
            "No individual @logins found in %s. Add reviewers explicitly to "
            "enable auto-assign.",
            codeowners_path,
        )
        set_gh_output("skip", "true")
        return 0

    reviewer = _pick_reviewer(candidates, excluded=[author, *already_requested])
    if not reviewer:
        return _skip_assign(
            f"No eligible reviewer found in {codeowners_path} (author={author})."
        )

    log.info("Selected reviewer: %s", reviewer)
    gh_pr_assign(repo, pr_number, reviewer)
    log.info(
        "Requested review and added assignee %s on #%d (via gh pr edit).",
        reviewer,
        pr_number,
    )

    if not (channel and payload_file):
        log.info(
            "SLACK_CHANNEL / SLACK_PAYLOAD_FILE not set; skipping Slack payload."
        )
        set_gh_output("skip", "true")
        return 0

    names = ReviewerDisplay(repo=repo)
    text = (
        f":eyes: New PR ready for review: "
        f"<{html_url}|#{pr_number} — {title}> by `@{author}` "
        f"— assigned to {names.name(reviewer)}"
    )
    write_slack_payload(payload_file, channel, text)
    log.info("Wrote Slack payload to %s", payload_file)
    set_gh_output("skip", "false")
    return 0


# ---------------------------------------------------------------------------
# Subcommand: remind
# ---------------------------------------------------------------------------


def _latest_review_state_per_user(reviews: list[dict]) -> dict[str, str]:
    """Return ``{login: latest_review_state}`` collapsing comments.

    ``COMMENTED`` reviews do not overwrite an earlier ``APPROVED`` /
    ``CHANGES_REQUESTED``, but a later ``DISMISSED`` does — i.e. dismissed
    approvals are treated as "no action taken".
    """
    by_user: dict[str, str] = {}
    for rv in reviews:
        user = (rv.get("user") or {}).get("login")
        state = rv.get("state")
        if not user or not state:
            continue
        if state == "PENDING":
            continue
        if state == "DISMISSED":
            by_user[user] = "DISMISSED"
            continue
        existing = by_user.get(user)
        if existing in ACTIONED_STATES and state == "COMMENTED":
            continue
        by_user[user] = state
    return by_user


def _classify_pr_for_reminder(
    pr: dict,
    reviews: list[dict],
    first_request_time: datetime | None = None,
    now: datetime | None = None,
) -> dict | None:
    states = _latest_review_state_per_user(reviews)
    if any(s in ACTIONED_STATES for s in states.values()):
        return None

    requested_users = sorted({u["login"] for u in pr.get("requested_reviewers", []) or []})
    commented_only = sorted({u for u, s in states.items() if s == "COMMENTED"})

    if not requested_users and not commented_only:
        return None

    if now is None:
        now = datetime.now(timezone.utc)

    waiting_since = first_request_time
    waiting_source = "review_requested"
    if waiting_since is None:
        created = pr.get("created_at")
        if created:
            try:
                waiting_since = datetime.strptime(
                    created, "%Y-%m-%dT%H:%M:%SZ"
                ).replace(tzinfo=timezone.utc)
                waiting_source = "pr_created"
            except ValueError:
                waiting_since = None

    waiting_hours: float | None = None
    if waiting_since is not None:
        waiting_hours = max(
            0.0, (now - waiting_since).total_seconds() / 3600.0
        )

    return {
        "number": pr["number"],
        "title": pr["title"],
        "author": pr["user"]["login"],
        "url": pr["html_url"],
        "updated_at": pr.get("updated_at", ""),
        "requested": requested_users,
        "commented_only": commented_only,
        "waiting_hours": waiting_hours,
        "waiting_source": waiting_source if waiting_hours is not None else None,
    }


def _format_waiting(entry: dict) -> str:
    hours = entry.get("waiting_hours")
    if hours is None:
        return ""
    total_minutes = int(round(hours * 60))
    days, rem_minutes = divmod(total_minutes, 24 * 60)
    rem_hours = rem_minutes // 60
    if days > 0:
        human = f"{days}d {rem_hours}h" if rem_hours else f"{days}d"
    elif rem_hours > 0:
        human = f"{rem_hours}h"
    else:
        human = "<1h"
    if entry.get("waiting_source") == "pr_created":
        return f"waiting {human} (no review requested yet)"
    return f"waiting {human} since review requested"


def _format_reminder_message(
    awaiting: list[dict], names: ReviewerDisplay
) -> str:
    lines = [
        f":alarm_clock: *{len(awaiting)} PR(s) waiting on a reviewer:*",
    ]
    for pr in awaiting:
        people = pr["requested"] + [
            u for u in pr["commented_only"] if u not in pr["requested"]
        ]
        people_str = (
            ", ".join(names.name(u) for u in people)
            if people
            else "_no reviewer assigned_"
        )
        waiting_suffix = _format_waiting(pr)
        head = (
            f"• <{pr['url']}|#{pr['number']} — {pr['title']}> "
            f"by `@{pr['author']}` — {people_str}"
        )
        if waiting_suffix:
            head += f" — _{waiting_suffix}_"
        lines.append(head)
        if pr["commented_only"]:
            commented_names = ", ".join(
                names.name(u) for u in pr["commented_only"]
            )
            lines.append(
                f"    _Note: {commented_names} left comment(s) — "
                f"comments do not count as a review._"
            )
    return "\n".join(lines)


def cmd_remind(args: argparse.Namespace) -> int:
    repo = os.environ["GH_REPO"]
    channel, payload_file = _slack_runtime()
    if not (channel and payload_file):
        log.error(
            "SLACK_CHANNEL and SLACK_PAYLOAD_FILE are required for remind mode"
        )
        return 2

    log.info("Listing open PRs in %s ...", repo)
    prs = list_open_prs(repo)
    log.info("Found %d open PR(s).", len(prs))

    now = datetime.now(timezone.utc)
    awaiting: list[dict] = []
    for pr in prs:
        if pr.get("draft"):
            continue
        reviews = list_pr_reviews(repo, pr["number"])
        first_request = first_review_request_time(repo, pr["number"])
        entry = _classify_pr_for_reminder(pr, reviews, first_request, now=now)
        if entry is not None:
            awaiting.append(entry)

    if not awaiting:
        log.info("No PRs need a reminder; not writing Slack payload.")
        set_gh_output("skip", "true")
        return 0

    awaiting.sort(
        key=lambda p: (p.get("waiting_hours") is None, -(p.get("waiting_hours") or 0.0))
    )
    names = ReviewerDisplay(repo=repo)
    text = _format_reminder_message(awaiting, names)
    write_slack_payload(payload_file, channel, text)
    log.info(
        "Wrote reminder digest for %d PR(s) to %s",
        len(awaiting),
        payload_file,
    )
    set_gh_output("skip", "false")
    return 0


# ---------------------------------------------------------------------------
# CLI plumbing
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("assign", help="Assign a random reviewer to one PR")
    sub.add_parser("remind", help="Build a reminder digest payload")
    args = parser.parse_args(argv)

    if args.cmd == "assign":
        return cmd_assign(args)
    if args.cmd == "remind":
        return cmd_remind(args)
    parser.error(f"Unknown command: {args.cmd}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
