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
    Requests review and adds the user as an assignee via the REST
    endpoints ``POST /pulls/:n/requested_reviewers`` and
    ``POST /issues/:n/assignees`` (we avoid ``gh pr edit`` because its
    GraphQL mutation fails on the deprecated ``projectCards`` field).
    Then writes a Slack payload describing the assignment to
    ``$SLACK_PAYLOAD_FILE`` so the next workflow step can post it via
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

``SLACK_BOT_TOKEN`` (optional but recommended)
    Slack bot token (``xoxb-...``) with the ``users:read.email`` scope.
    When set, the bot calls ``users.lookupByEmail`` once per *distinct*
    reviewer per run to translate their git commit email into a Slack
    user ID, emitting ``<@U…>`` so Slack renders a real channel mention
    that triggers a notification for the reviewer — same UX as typing
    ``@first.last`` manually in ``#drivers-review``. The bot only writes
    via ``chat.postMessage`` to a channel; it never sends DMs.
    ``users.list`` is intentionally *not* called — per-reviewer lookup
    fetches one user's profile at a time instead of leaking the whole
    workspace onto a public runner.

    If the token is not provided or the lookup fails for a given
    reviewer (e.g. they commit with a noreply email or aren't in the
    workspace), the message falls back to plain ``@handle`` text
    derived from their commit author name.

Assign-only environment variables
---------------------------------
``PR_NUMBER``
    Pull request number to operate on.

``CODEOWNERS_PATH`` (optional)
    Path to the CODEOWNERS file to parse. Defaults to
    ``.github/CODEOWNERS``.

Reviewer display names
----------------------
For each reviewer the bot:

1. Fetches their most recent commit in this repo to get both the git
   ``commit.author.name`` (e.g. ``Maxymilian Kowalski``) and the
   ``commit.author.email`` (e.g. ``maxymilian.kowalski@snowflake.com``).
2. If ``SLACK_BOT_TOKEN`` is configured, calls ``users.lookupByEmail``
   with the email and emits ``<@U…>`` on a successful match — this is
   what makes Slack render a real channel mention and notify the
   reviewer. The message itself is always posted in the channel; the
   bot never DMs anyone.
3. Otherwise, shows the commit author name as plain text
   (e.g. ``Maxymilian Kowalski``). This is *not* a clickable mention —
   Slack won't auto-link individual users — but the reviewer is still
   notified by GitHub via the ``review_requested`` API call we make
   immediately before posting, so the Slack text is purely
   informational.
4. As a last resort, falls back to ``@<github-login>`` when the user
   has no commits in the repo.

Email is used only as a lookup key for the Slack ID. It is never
written into any user-visible message.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import random
import subprocess
import sys
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
    """Request review and add assignee on a PR via REST endpoints.

    We deliberately avoid ``gh pr edit`` here: its underlying GraphQL
    mutation reads the now-deprecated ``projectCards`` field, which
    causes the call to fail with::

        GraphQL: Projects (classic) is being deprecated ...
        (repository.pullRequest.projectCards)

    on any repo that has migrated to the new Projects experience — even
    when the edit has nothing to do with projects. The REST endpoints
    used below don't go through that mutation and are unaffected.
    """
    _gh(
        "api",
        "--method",
        "POST",
        f"/repos/{repo}/pulls/{pr_number}/requested_reviewers",
        "-f",
        f"reviewers[]={login}",
    )
    _gh(
        "api",
        "--method",
        "POST",
        f"/repos/{repo}/issues/{pr_number}/assignees",
        "-f",
        f"assignees[]={login}",
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


def github_commit_author(repo: str, login: str) -> tuple[str | None, str | None]:
    """Return ``(name, email)`` from the user's most recent commit in *repo*.

    Both fields come from the git ``Author: Name <email>`` line of the
    user's most recent commit in this repo. ``(None, None)`` is returned
    when the user has no commits here or the API request fails.

    The email is used internally to resolve the user to a Slack ID via
    ``users.lookupByEmail`` (see :func:`slack_lookup_by_email`); it is
    never rendered into messages — the display name comes from the
    ``name`` field.
    """
    try:
        data = gh_api_json(
            f"/repos/{repo}/commits?author={login}&per_page=1"
        )
    except RuntimeError:
        return (None, None)
    if not isinstance(data, list) or not data:
        return (None, None)
    commit = (data[0] or {}).get("commit") or {}
    author = commit.get("author") or {}
    name = (author.get("name") or "").strip() or None
    email = (author.get("email") or "").strip() or None
    return (name, email)


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


# Emails that will never resolve in Slack — skip the network call.
_NORESOLVE_EMAIL_SUFFIXES = (
    "@users.noreply.github.com",
    "@noreply.github.com",
)


def slack_lookup_by_email(email: str, token: str) -> str | None:
    """Return the Slack user ID for *email* via ``users.lookupByEmail``.

    Used to translate a reviewer's git commit email to their Slack ID
    so we can emit a real ``<@U…>`` mention. We deliberately do *not*
    call ``users.list`` — fetching every workspace member onto a public
    runner just to identify 1–3 reviewers would be both wasteful and a
    PII exposure issue.

    Returns ``None`` on any failure (missing token, network error,
    ``users_not_found``, etc.) so the caller can fall back to plain
    ``@handle`` text. ``users_not_found`` is logged at INFO since it is
    the expected outcome for noreply / external committers; other Slack
    errors are logged at WARNING so misconfiguration (e.g. missing
    ``users:read.email`` scope) is visible.

    The Slack endpoint is invoked via ``curl`` (always present on the
    GitHub Actions runner) to stay consistent with our subprocess-based
    ``gh`` usage; no extra Python HTTP client is introduced.
    """
    if not email or not token:
        return None
    lowered = email.lower()
    if any(lowered.endswith(suffix) for suffix in _NORESOLVE_EMAIL_SUFFIXES):
        log.info("Skipping Slack lookup for noreply email %s.", email)
        return None

    try:
        result = subprocess.run(
            [
                "curl",
                "-sS",
                "--max-time",
                "10",
                "-G",
                "https://slack.com/api/users.lookupByEmail",
                "--data-urlencode",
                f"email={email}",
                "-H",
                f"Authorization: Bearer {token}",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as e:
        log.warning("curl not available, cannot resolve %s: %s", email, e)
        return None

    if result.returncode != 0:
        log.warning(
            "users.lookupByEmail curl failed (exit %d): %s",
            result.returncode,
            result.stderr.strip(),
        )
        return None

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        log.warning(
            "users.lookupByEmail returned non-JSON for %s: %r",
            email,
            result.stdout[:200],
        )
        return None

    if not data.get("ok"):
        err = data.get("error", "unknown")
        if err == "users_not_found":
            log.info(
                "No Slack workspace member with email %s; will render as "
                "plain @handle text.",
                email,
            )
        else:
            log.warning(
                "users.lookupByEmail error for %s: %s (check the "
                "users:read.email scope on SLACK_BOT_TOKEN).",
                email,
                err,
            )
        return None

    return ((data.get("user") or {}).get("id")) or None


class ReviewerDisplay:
    """Cache GitHub login -> Slack mention string.

    Resolution order, per reviewer:

    1. Fetch the user's most recent commit in this repo to get their
       ``commit.author.name`` and ``commit.author.email``.
    2. If a Slack bot token is configured, call ``users.lookupByEmail``
       with that email. On a match, return ``<@U…>`` so Slack renders a
       real channel mention that notifies the reviewer (the message is
       posted in the channel — the bot never sends DMs).
    3. Otherwise (no token, no email, lookup failed, or
       ``users_not_found``), fall back to the plain commit author name
       (e.g. ``Maxymilian Kowalski``). This is *not* a clickable mention
       — it's purely informational — but reads better than a synthetic
       ``@first.last`` handle that Slack would not auto-resolve anyway.
    4. If we have no commit author name either, fall back to
       ``@<github-login>``.

    Step (2) is best-effort and only touches Slack once per *distinct*
    reviewer per run (results are cached). The bot does not call
    ``users.list``.
    """

    def __init__(
        self,
        repo: str | None,
        slack_token: str | None = None,
    ) -> None:
        self._repo = repo
        self._slack_token = slack_token
        self._cache: dict[str, str] = {}

    def name(self, login: str) -> str:
        if not login:
            return ""
        if login in self._cache:
            return self._cache[login]

        commit_name: str | None = None
        commit_email: str | None = None
        if self._repo:
            commit_name, commit_email = github_commit_author(self._repo, login)

        if commit_email and self._slack_token:
            slack_uid = slack_lookup_by_email(commit_email, self._slack_token)
            if slack_uid:
                mention = f"<@{slack_uid}>"
                log.info(
                    "Resolved %s -> <@%s> via users.lookupByEmail (%s).",
                    login,
                    slack_uid,
                    commit_email,
                )
                self._cache[login] = mention
                return mention

        if commit_name:
            log.info(
                "No Slack lookup match for %s; using commit author name %r.",
                login,
                commit_name,
            )
            self._cache[login] = commit_name
            return commit_name

        mention = f"@{login}"
        log.info(
            "No commit author name for %s in %s; falling back to GitHub login.",
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


def write_slack_payload(
    path: Path,
    channel: str,
    text: str,
    blocks: list[dict] | None = None,
) -> None:
    """Write a ``chat.postMessage`` payload to *path*.

    *text* is always set — Slack uses it as the fallback in notifications
    and screen readers. *blocks*, when given, drives the in-channel
    layout via Block Kit and is the preferred way to format multi-line
    messages with sections, dividers and context elements.

    ``link_names`` is kept on so that ``@channel`` / ``@here`` and any
    workspace user groups (e.g. ``@drivers-warsaw``) are auto-resolved
    in the fallback text. Individual users cannot be mentioned by
    ``@first.last`` — Slack requires ``<@U…>`` for that — so the
    handles we emit (e.g. ``@maxymilian.kowalski``) render as plain
    text. GitHub already notifies the reviewer via the API call.
    """
    payload: dict[str, Any] = {
        "channel": channel,
        "text": text,
        "link_names": True,
        "unfurl_links": False,
        "unfurl_media": False,
    }
    if blocks:
        payload["blocks"] = blocks
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

    names = ReviewerDisplay(
        repo=repo,
        slack_token=os.environ.get("SLACK_BOT_TOKEN") or None,
    )
    reviewer_display = names.name(reviewer)
    fallback = (
        f"New PR ready for review: #{pr_number} — {title} "
        f"(by @{author}, reviewer {reviewer_display})"
    )
    blocks: list[dict] = [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": ":eyes: *New PR ready for review*",
            },
        },
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": f"<{html_url}|#{pr_number} — {title}>",
            },
        },
        {
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": (
                        f"Author: `@{author}`   ·   "
                        f"Reviewer: {reviewer_display}"
                    ),
                }
            ],
        },
    ]
    write_slack_payload(payload_file, channel, fallback, blocks=blocks)
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


def _reminder_fallback_text(awaiting: list[dict]) -> str:
    """Plain-text summary used as the Slack notification fallback."""
    return f"{len(awaiting)} PR(s) waiting on a reviewer"


def _build_reminder_blocks(
    awaiting: list[dict], names: ReviewerDisplay
) -> list[dict]:
    """Return a Block Kit payload listing every PR awaiting a reviewer.

    The layout is one ``section`` (with the linked PR title) plus one
    ``context`` element (with author, reviewer(s) and waiting time) per
    PR, separated by ``divider`` blocks. A heading section announces
    the count.
    """
    blocks: list[dict] = [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": (
                    f":alarm_clock: *{len(awaiting)} PR(s) waiting on a "
                    f"reviewer*"
                ),
            },
        },
        {"type": "divider"},
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
        meta_parts = [
            f"Author: `@{pr['author']}`",
            f"Reviewer{'s' if len(people) > 1 else ''}: {people_str}",
        ]
        waiting_suffix = _format_waiting(pr)
        if waiting_suffix:
            meta_parts.append(f"_{waiting_suffix}_")
        meta_text = "   ·   ".join(meta_parts)

        if pr["commented_only"]:
            commented_names = ", ".join(
                names.name(u) for u in pr["commented_only"]
            )
            meta_text += (
                f"\n_Note: {commented_names} left comment(s) — "
                f"comments do not count as a review._"
            )

        blocks.append(
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": f"<{pr['url']}|#{pr['number']} — {pr['title']}>",
                },
            }
        )
        blocks.append(
            {
                "type": "context",
                "elements": [{"type": "mrkdwn", "text": meta_text}],
            }
        )

    return blocks


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
    names = ReviewerDisplay(
        repo=repo,
        slack_token=os.environ.get("SLACK_BOT_TOKEN") or None,
    )
    fallback = _reminder_fallback_text(awaiting)
    blocks = _build_reminder_blocks(awaiting, names)
    write_slack_payload(payload_file, channel, fallback, blocks=blocks)
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
