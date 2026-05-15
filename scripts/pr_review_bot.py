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
    Pick a random reviewer for a single PR.

    The candidate pool is derived from ``.github/reviewers`` (or
    another path via ``REVIEWERS_PATH``) keyed by *PR label*: each
    rule has the form ``<label> @login1 @login2 ...`` and contributes
    its reviewers when the PR carries that label. Reviewers are
    unioned across every matched rule so a PR labeled both ``python``
    and ``odbc`` pools the experts of each domain. The reserved key
    ``all`` (see :data:`FALLBACK_KEY`) defines the fallback pool used
    when *no* PR label matches a rule (e.g. a fresh PR opened before
    the triage labels are applied). Labels are read straight from the
    PR payload — no extra API call is required.

    The PR author and any user already requested for review are
    excluded from the pool. Candidates whose Slack status currently
    signals out-of-office (see :data:`_OOO_STATUS_EMOJIS` /
    :data:`_OOO_TEXT_REGEX`) are dropped before the random pick —
    unless the OOO filter would empty the pool, in which case the
    unfiltered list is used so the PR is never left without a
    reviewer. Status comes from ``users.info`` and requires
    ``SLACK_BOT_TOKEN`` with the ``users:read`` scope (granted
    implicitly by ``users:read.email``); when the token is missing or
    the lookup fails the filter no-ops silently. As a final
    safety-net, if no PR label matches any rule *and* the rules file
    has no ``all`` fallback, the bot widens the pool to the union of
    every listed reviewer.

    Requests review and adds the user as an assignee via the REST
    endpoints ``POST /pulls/:n/requested_reviewers`` and
    ``POST /issues/:n/assignees`` (we avoid ``gh pr edit`` because its
    GraphQL mutation fails on the deprecated ``projectCards`` field).
    Then writes a Slack payload describing the assignment to
    ``$SLACK_PAYLOAD_FILE`` so the next workflow step can post it via
    ``slackapi/slack-github-action``.

    Designed to be run from a ``pull_request_target`` workflow on the
    ``opened`` and ``ready_for_review`` activity types. Drafts are skipped.

``remind``
    Iterate every open non-draft PR in the repository (via ``gh``) and
    write a digest Slack payload listing PRs that are *waiting on a
    reviewer's action* — i.e. no review with state ``APPROVED`` or
    ``CHANGES_REQUESTED``. PRs where a requested reviewer has only
    ``COMMENTED`` are flagged with a note that comments do not count as a
    review. Each entry includes the time elapsed since the *initial*
    ``review_requested`` event.

    Scheduled posts (``GH_EVENT_NAME=schedule``) are suppressed during
    Warsaw quiet hours (``QUIET_HOURS_START``..``QUIET_HOURS_END``,
    i.e. 17:00–07:59 ``Europe/Warsaw``) because nobody on the team
    reads pings overnight. The guard is DST-aware via
    :mod:`zoneinfo`. The workflow cron is already narrowed to UTC
    hours that overlap Warsaw 08:00–16:59 in both CET and CEST so the
    runner is not spun up during dead hours; the Python guard exists
    to catch the DST boundary slots that fall on the edge. Manual
    ``workflow_dispatch`` runs bypass the guard so on-call folks can
    always trigger a digest.

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

``REVIEWERS_PATH`` (optional)
    Path to the reviewer-pool file to parse. Defaults to
    ``.github/reviewers``.

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
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable
from zoneinfo import ZoneInfo

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
log = logging.getLogger("pr-review-bot")

DEFAULT_REVIEWERS_PATH = Path(".github/reviewers")

# Review states that mean the reviewer has taken action on the PR.
ACTIONED_STATES = {"APPROVED", "CHANGES_REQUESTED"}

# Scheduled reminder posts are suppressed outside of Warsaw working hours.
# The team is in Europe/Warsaw; nobody reads pings between 17:00 and the
# next morning, and a digest sitting in the channel overnight gets buried
# under whatever lands first thing in the morning instead of catching
# attention. The guard is applied per-run (DST-correct via zoneinfo);
# the schedule cron is also narrowed to UTC hours that *can* fall inside
# this window in either CET or CEST so we don't even spin up the runner
# during dead hours. Manual `workflow_dispatch` runs ignore the guard.
WARSAW_TZ = ZoneInfo("Europe/Warsaw")
QUIET_HOURS_START = 17  # 17:00 Warsaw — first hour we suppress.
QUIET_HOURS_END = 8     # 08:00 Warsaw — first hour we resume.

# Slack status emojis that, when set on a reviewer's profile, signal
# unavailability. Conservative list — extend as the team converges on
# conventions. Lowercased for case-insensitive comparison.
_OOO_STATUS_EMOJIS = frozenset({
    ":palm_tree:",
    ":beach:",
    ":beach_with_umbrella:",
    ":airplane:",
    ":airplane_departure:",
    ":airplane_arriving:",
    ":no_entry_sign:",
    ":zzz:",
    ":hospital:",
    ":face_with_thermometer:",
    ":vacation:",
})

# Free-text patterns that signal OOO when present in a Slack status_text.
# Matched against the lowercased status text with simple word boundaries
# so e.g. "google" does not false-match "ooo". The token "ooo" itself is
# a real-world OOO abbreviation so we accept it standalone.
_OOO_TEXT_REGEX = re.compile(
    r"\b("
    r"ooo|"
    r"out[ -]of[ -]office|"
    r"pto|"
    r"vacation|"
    r"holiday|"
    r"afk|"
    r"away until|"
    r"on leave"
    r")\b",
    re.IGNORECASE,
)


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


def gh_pr_remove_reviewer(repo: str, pr_number: int, login: str) -> None:
    """Remove *login* from the requested-reviewers list of *pr_number*.

    Used when the PR author themselves ends up requested as a reviewer
    (e.g. via GitHub's team-based code-owner round-robin or a manual
    request), so the bot can guarantee an author is never one of their
    own reviewers. The REST endpoint is idempotent — a 422 for a user
    who isn't currently requested is silently ignored by ``gh``.
    """
    _gh(
        "api",
        "--method",
        "DELETE",
        f"/repos/{repo}/pulls/{pr_number}/requested_reviewers",
        "-f",
        f"reviewers[]={login}",
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


def _is_bot_user(user: dict | None) -> bool:
    """Return True when *user* (a GitHub user payload) is a bot account.

    Used to exclude bot reviewers (Copilot, Dependabot, Renovate, …)
    from anywhere a real person is implied — they should not be
    pinged in Slack, should not gate the "already has reviewers"
    skip in :func:`cmd_assign`, and should not let a bot approval
    count as the PR having been actioned.

    Both signals are checked because they fail in opposite ways:

    * ``user["type"] == "Bot"`` — the canonical API discriminator,
      but older webhook payloads occasionally omit ``type`` on the
      user object inside ``requested_reviewers``.
    * ``login.endswith("[bot]")`` — the rendered convention every GH
      app uses (e.g. ``copilot-pull-request-reviewer[bot]``); always
      present even when ``type`` is missing.
    """
    if not isinstance(user, dict):
        return False
    if user.get("type") == "Bot":
        return True
    login = user.get("login") or ""
    return login.endswith("[bot]")


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
# Reviewer pool parsing (PR-label keyed)
# ---------------------------------------------------------------------------

# Parsed reviewer pool: ordered list of (label, [logins]) rules.
# ``label`` is the PR-label name the rule applies to. The reserved key
# ``FALLBACK_KEY`` (``"all"``) names the fallback bucket — used when no
# PR label matches any rule. It is deliberately a real-looking label
# (instead of e.g. ``*``) so the rules file reads as a uniform list of
# labels. We assume no PR is ever tagged with ``all``; if one is, that
# rule's reviewers would simply also fire — semantically harmless.
FALLBACK_KEY = "all"
ReviewerRules = list[tuple[str, list[str]]]


def parse_reviewers(path: Path = DEFAULT_REVIEWERS_PATH) -> ReviewerRules:
    """Parse the bot's reviewer pool, keyed by PR label.

    Each non-empty, non-comment line is::

        <label>   @login1 @login2 ...

    where ``<label>`` matches a GitHub label name on the PR
    (case-insensitively) — e.g. ``jdbc``, ``odbc``, ``python``,
    ``nodejs``. The reserved key ``all`` (see :data:`FALLBACK_KEY`)
    defines the fallback pool used when *no* PR label matches a rule.
    Blank lines and ``#`` comments are ignored. Team references
    (``@org/team-slug``) are rejected — the bot deliberately avoids
    the ``read:org`` scope.

    Returns an ordered ``list[tuple[label, [logins]]]``. Order is
    preserved purely for stable logging / deterministic random pick;
    label matching itself is set-based.
    """
    if not path.exists():
        raise FileNotFoundError(f"Reviewers file not found at {path}")

    rules: ReviewerRules = []

    for lineno, raw_line in enumerate(path.read_text().splitlines(), start=1):
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        tokens = line.split()
        label, owner_tokens = tokens[0], tokens[1:]
        owners: list[str] = []
        for tok in owner_tokens:
            if not tok.startswith("@"):
                log.warning(
                    "Ignoring malformed owner %r on line %d of %s (expected @login).",
                    tok,
                    lineno,
                    path,
                )
                continue
            handle = tok[1:]
            if "/" in handle:
                log.warning(
                    "Ignoring team reference %r on line %d of %s; "
                    "list individual @logins only.",
                    tok,
                    lineno,
                    path,
                )
                continue
            owners.append(handle)

        # Dedupe within a single line while preserving order, in case the
        # file accidentally lists the same person twice on one rule.
        deduped: list[str] = []
        seen: set[str] = set()
        for handle in owners:
            key = handle.lower()
            if key in seen:
                continue
            seen.add(key)
            deduped.append(handle)
        rules.append((label, deduped))

    return rules


def select_candidates(
    rules: ReviewerRules, labels: Iterable[str]
) -> list[str]:
    """Return the union of reviewers whose rule label is on the PR.

    Matching is case-insensitive. Reviewers from every matched rule
    are unioned (a PR labeled both ``python`` and ``odbc`` pools the
    experts from both rules). When no PR label matches any rule the
    reviewers of the :data:`FALLBACK_KEY` rule (``all``) are returned
    instead — these are the generalists who can review anything.

    The returned list preserves rules-file order, so the downstream
    random pick depends only on the RNG, not on dict iteration order.
    Returns an empty list only when *neither* a label matched *nor* an
    ``all`` fallback is configured; :func:`cmd_assign` widens to the
    full pool in that case.
    """
    label_set = {(l or "").lower() for l in labels if l}

    selected: list[str] = []
    seen: set[str] = set()
    fallback_owners: list[str] = []

    for key, owners in rules:
        if key == FALLBACK_KEY:
            fallback_owners = owners
            continue
        if key.lower() not in label_set:
            continue
        for login in owners:
            k = login.lower()
            if k in seen:
                continue
            seen.add(k)
            selected.append(login)

    if selected:
        return selected

    for login in fallback_owners:
        k = login.lower()
        if k in seen:
            continue
        seen.add(k)
        selected.append(login)
    return selected


def all_reviewers(rules: ReviewerRules) -> list[str]:
    """Return every unique reviewer login across all rules.

    Used as the last-resort candidate pool when no rule label matches
    a PR's labels *and* the file has no ``all`` fallback configured.
    Order matches the rules-file order; duplicates across rules are
    collapsed.
    """
    flat: list[str] = []
    seen: set[str] = set()
    for _label, owners in rules:
        for handle in owners:
            key = handle.lower()
            if key in seen:
                continue
            seen.add(key)
            flat.append(handle)
    return flat


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


def slack_user_status(uid: str, token: str) -> dict | None:
    """Return ``{emoji, text, expiration}`` for *uid* via ``users.info``.

    Used to decide whether a reviewer is currently out of office. Reads
    only ``profile.status_emoji`` / ``profile.status_text`` /
    ``profile.status_expiration`` from the response — no other profile
    fields are inspected.

    Requires the ``users:read`` scope on the bot token. Slack grants
    ``users:read`` implicitly when ``users:read.email`` is granted, so
    no scope change is needed beyond what the assign + reminder paths
    already require. ``None`` is returned on any failure (network,
    auth, parse, ``user_not_found``) so the caller can gracefully
    skip OOO filtering instead of crashing.
    """
    if not uid or not token:
        return None
    try:
        result = subprocess.run(
            [
                "curl",
                "-sS",
                "--max-time",
                "10",
                "-G",
                "https://slack.com/api/users.info",
                "--data-urlencode",
                f"user={uid}",
                "-H",
                f"Authorization: Bearer {token}",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as e:
        log.warning("curl not available, cannot fetch status for %s: %s", uid, e)
        return None

    if result.returncode != 0:
        log.warning(
            "users.info curl failed (exit %d): %s",
            result.returncode,
            result.stderr.strip(),
        )
        return None

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        log.warning(
            "users.info returned non-JSON for %s: %r",
            uid,
            result.stdout[:200],
        )
        return None

    if not data.get("ok"):
        err = data.get("error", "unknown")
        log.info(
            "users.info error for %s: %s (OOO filter will treat as available).",
            uid,
            err,
        )
        return None

    profile = (data.get("user") or {}).get("profile") or {}
    return {
        "emoji": (profile.get("status_emoji") or "").strip(),
        "text": (profile.get("status_text") or "").strip(),
        "expiration": int(profile.get("status_expiration") or 0),
    }


def _is_ooo(status: dict | None, now_ts: float | None = None) -> bool:
    """Return True when a Slack status indicates the user is OOO.

    ``status`` is the dict returned by :func:`slack_user_status`
    (``None`` if the lookup failed — treated as "available").

    A status is considered OOO when its emoji is in
    ``_OOO_STATUS_EMOJIS`` *or* its text matches ``_OOO_TEXT_REGEX``,
    AND the status hasn't expired yet (``status_expiration == 0`` or
    in the future).
    """
    if not status:
        return False
    expiration = status.get("expiration") or 0
    if expiration:
        if now_ts is None:
            now_ts = datetime.now(timezone.utc).timestamp()
        if expiration < now_ts:
            return False
    emoji = (status.get("emoji") or "").lower()
    if emoji in _OOO_STATUS_EMOJIS:
        return True
    text = status.get("text") or ""
    if text and _OOO_TEXT_REGEX.search(text):
        return True
    return False


class ReviewerDisplay:
    """Cache GitHub login -> Slack mention string + OOO state.

    Resolution order, per reviewer:

    1. Fetch the user's most recent commit in this repo to get their
       ``commit.author.name`` and ``commit.author.email``.
    2. If a Slack bot token is configured, call ``users.lookupByEmail``
       with that email. On a match, return ``<@U…>`` so Slack renders a
       real channel mention that notifies the reviewer (the message is
       posted in the channel — the bot never sends DMs). Also fetch
       ``users.info`` for the same user and check for an OOO status
       (see :func:`_is_ooo`); the result is cached so each reviewer is
       only checked once per workflow run.
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
        # login -> {"mention": str, "ooo": bool, "ooo_emoji": str | None}
        self._cache: dict[str, dict] = {}

    def _resolve(self, login: str) -> dict:
        if not login:
            return {"mention": "", "ooo": False, "ooo_emoji": None}
        cached = self._cache.get(login)
        if cached is not None:
            return cached

        commit_name: str | None = None
        commit_email: str | None = None
        if self._repo:
            commit_name, commit_email = github_commit_author(self._repo, login)

        mention: str | None = None
        ooo = False
        ooo_emoji: str | None = None

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
                status = slack_user_status(slack_uid, self._slack_token)
                if _is_ooo(status):
                    ooo = True
                    ooo_emoji = (status or {}).get("emoji") or None
                    log.info(
                        "Reviewer %s is OOO (emoji=%r, text=%r).",
                        login,
                        (status or {}).get("emoji", ""),
                        (status or {}).get("text", ""),
                    )

        if mention is None:
            if commit_name:
                log.info(
                    "No Slack lookup match for %s; using commit author name %r.",
                    login,
                    commit_name,
                )
                mention = commit_name
            else:
                mention = f"@{login}"
                log.info(
                    "No commit author name for %s in %s; falling back to GitHub login.",
                    login,
                    self._repo,
                )

        entry = {"mention": mention, "ooo": ooo, "ooo_emoji": ooo_emoji}
        self._cache[login] = entry
        return entry

    def name(self, login: str) -> str:
        return self._resolve(login)["mention"]

    def is_ooo(self, login: str) -> bool:
        return bool(self._resolve(login)["ooo"])

    def ooo_emoji(self, login: str) -> str | None:
        return self._resolve(login)["ooo_emoji"]


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


def _filter_ooo(
    candidates: list[str], names: ReviewerDisplay
) -> list[str]:
    """Drop candidates whose Slack status currently signals OOO.

    Falls back to the unfiltered pool when *every* candidate appears
    OOO (or status lookups failed) so a PR is never left unassigned.
    Cheap on repeat calls — ``ReviewerDisplay`` caches per login.
    """
    available = [c for c in candidates if not names.is_ooo(c)]
    if not available:
        log.info(
            "All %d candidate(s) appear OOO or Slack status unavailable; "
            "falling back to the full pool so the PR isn't left unassigned.",
            len(candidates),
        )
        return list(candidates)
    skipped = [c for c in candidates if names.is_ooo(c)]
    if skipped:
        log.info(
            "OOO filter dropped %d candidate(s): %s",
            len(skipped),
            ", ".join(skipped),
        )
    return available


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
    reviewers_path = Path(
        os.environ.get("REVIEWERS_PATH") or DEFAULT_REVIEWERS_PATH
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

    # `requested_reviewers` can pick up entries we don't want gating the
    # skip-or-assign decision:
    #
    # 1. The PR author themselves. GitHub's team-based code-owner
    #    round-robin (CODEOWNERS points at @snowflakedb/snow-drivers-warsaw)
    #    can pick the author, and a human can also request them manually;
    #    in both cases we strip them and call DELETE so the GH UI matches.
    # 2. Bot reviewers (Copilot, Dependabot, …) that get attached
    #    automatically. They are not people, so a Copilot-only review
    #    request must not block this job from picking a human.
    requested_objs = [
        u for u in pr.get("requested_reviewers", []) or [] if isinstance(u, dict)
    ]
    all_requested_logins = [u["login"] for u in requested_objs if u.get("login")]
    if any(u.lower() == author.lower() for u in all_requested_logins):
        log.warning(
            "PR #%d had its own author (%s) listed as a requested reviewer; "
            "removing.",
            pr_number,
            author,
        )
        try:
            gh_pr_remove_reviewer(repo, pr_number, author)
        except RuntimeError as e:
            log.warning("Failed to remove author from reviewers: %s", e)
    bot_reviewers = [u["login"] for u in requested_objs if _is_bot_user(u)]
    if bot_reviewers:
        log.info(
            "Ignoring %d bot reviewer(s) on PR #%d: %s",
            len(bot_reviewers),
            pr_number,
            ", ".join(bot_reviewers),
        )
    already_requested = [
        u["login"]
        for u in requested_objs
        if not _is_bot_user(u) and u["login"].lower() != author.lower()
    ]
    if already_requested:
        return _skip_assign(
            f"PR #{pr_number} already has reviewers requested "
            f"({', '.join(already_requested)}); skipping auto-assign."
        )

    log.info("Reading reviewer pool from %s ...", reviewers_path)
    try:
        rules = parse_reviewers(reviewers_path)
    except FileNotFoundError as e:
        log.error("%s. Aborting assign.", e)
        return 1

    full_pool = all_reviewers(rules)
    log.info(
        "%s defines %d rule(s) covering %d unique reviewer(s).",
        reviewers_path,
        len(rules),
        len(full_pool),
    )
    if not full_pool:
        log.warning(
            "No individual @logins found in %s. Add reviewers explicitly to "
            "enable auto-assign.",
            reviewers_path,
        )
        set_gh_output("skip", "true")
        return 0

    # Label-based narrowing: prefer the experts whose rule label is on
    # this PR. Labels live on the PR payload we already fetched, so no
    # extra API call is needed. When no rule matches *and* there is no
    # ``all`` fallback in the rules file, widen the pool to every
    # listed reviewer so the PR is never left without a candidate.
    pr_labels = [
        (lbl.get("name") or "").strip()
        for lbl in pr.get("labels", []) or []
        if isinstance(lbl, dict)
    ]
    pr_labels = [lbl for lbl in pr_labels if lbl]
    log.info(
        "PR #%d carries %d label(s): %s",
        pr_number,
        len(pr_labels),
        ", ".join(pr_labels) if pr_labels else "(none)",
    )
    candidates = select_candidates(rules, pr_labels)
    if candidates:
        log.info(
            "Label-based selection picked %d candidate(s): %s",
            len(candidates),
            ", ".join(candidates),
        )
    else:
        log.warning(
            "No rule in %s matched any PR label and no `%s` fallback is "
            "configured; widening to the full reviewer pool.",
            reviewers_path,
            FALLBACK_KEY,
        )
        candidates = full_pool

    # Build the display/OOO resolver early so we can pre-filter the
    # candidate pool by Slack status. The resolver caches per-login
    # lookups, so reusing it later for the picked reviewer's mention
    # doesn't re-hit Slack.
    names = ReviewerDisplay(
        repo=repo,
        slack_token=os.environ.get("SLACK_BOT_TOKEN") or None,
    )
    available_candidates = _filter_ooo(candidates, names)

    reviewer = _pick_reviewer(
        available_candidates, excluded=[author, *already_requested]
    )
    if not reviewer:
        return _skip_assign(
            f"No eligible reviewer found in {reviewers_path} (author={author})."
        )

    log.info("Selected reviewer: %s (author %s excluded)", reviewer, author)
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

    Bot reviews are ignored entirely: a Copilot approval doesn't count
    as the PR having been actioned, and a Copilot comment shouldn't
    sneak the bot's login into the displayed ``commented_only`` list.
    """
    by_user: dict[str, str] = {}
    for rv in reviews:
        user_obj = rv.get("user") or {}
        if _is_bot_user(user_obj):
            continue
        user = user_obj.get("login")
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

    # Strip two kinds of would-be reviewers from the displayed lists:
    #
    # 1. The PR author themselves — a self "Comment review" or a
    #    misfired `requested_reviewers` entry from GitHub's team
    #    round-robin should never surface them as someone we're
    #    waiting on.
    # 2. Bot reviewers (Copilot, Dependabot, …) — not people; naming
    #    them in a Slack nudge confuses the channel. ``commented_only``
    #    is derived from ``states``, which is already bot-free thanks
    #    to :func:`_latest_review_state_per_user`.
    author_login = ((pr.get("user") or {}).get("login") or "").lower()
    requested_users = sorted(
        {
            u["login"]
            for u in pr.get("requested_reviewers", []) or []
            if isinstance(u, dict)
            and u.get("login")
            and not _is_bot_user(u)
            and u["login"].lower() != author_login
        }
    )
    commented_only = sorted(
        {u for u, s in states.items() if s == "COMMENTED" and u.lower() != author_login}
    )

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


def _format_waiting_compact(entry: dict) -> str:
    """Single-token waiting time for the bullet-list reminder digest.

    Examples: ``"2d"``, ``"5h"``, ``"<1h"``. Returns ``""`` when the
    entry has no usable waiting timestamp.
    """
    hours = entry.get("waiting_hours")
    if hours is None:
        return ""
    total_minutes = int(round(hours * 60))
    days = total_minutes // (24 * 60)
    if days >= 1:
        return f"{days}d"
    rem_hours = total_minutes // 60
    if rem_hours >= 1:
        return f"{rem_hours}h"
    return "<1h"


def _reminder_fallback_text(awaiting: list[dict]) -> str:
    """Plain-text summary used as the Slack notification fallback."""
    return f"{len(awaiting)} PR(s) waiting on a reviewer"


def _is_warsaw_quiet_hours(now: datetime | None = None) -> bool:
    """Return True when *now* falls in the Warsaw 17:00–07:59 quiet window.

    DST-correct: the comparison is done after converting to
    ``Europe/Warsaw``, so a UTC 06:00 fires from a cron entry resolves
    to 07:00 in winter (suppressed) and 08:00 in summer (allowed).

    Args:
        now: Override the wall clock. Defaults to ``datetime.now(timezone.utc)``;
            the override exists for testability.
    """
    if now is None:
        now = datetime.now(timezone.utc)
    warsaw_hour = now.astimezone(WARSAW_TZ).hour
    return warsaw_hour >= QUIET_HOURS_START or warsaw_hour < QUIET_HOURS_END


def _decorate_reviewer(names: ReviewerDisplay, login: str) -> str:
    """Render a reviewer mention, appending an OOO marker when applicable.

    Uses the reviewer's own Slack status emoji when one is configured
    (so a `:palm_tree:` user shows up as ``<@U…> :palm_tree:``); falls
    back to ``:zzz:`` when the OOO heuristic matched on status_text but
    no emoji was set. Non-OOO reviewers render unchanged.
    """
    label = names.name(login)
    if not names.is_ooo(login):
        return label
    emoji = names.ooo_emoji(login) or ":zzz:"
    return f"{label} {emoji}"


def _build_reminder_blocks(
    awaiting: list[dict], names: ReviewerDisplay
) -> list[dict]:
    """Return a Block Kit payload listing every PR awaiting a reviewer.

    Compact layout: one heading section plus one bulleted section
    where each line is ``• <url|title> — reviewer(s) — waiting``. Keeps
    the digest short even when many PRs are stale; per-PR author and
    "comments don't count" footnotes are dropped on purpose — the linked
    PR carries the rest of the context for anyone who clicks through.
    """
    lines: list[str] = []
    for pr in awaiting:
        people = pr["requested"] + [
            u for u in pr["commented_only"] if u not in pr["requested"]
        ]
        people_str = (
            ", ".join(_decorate_reviewer(names, u) for u in people)
            if people
            else "_no reviewer_"
        )
        waiting = _format_waiting_compact(pr)
        parts = [f"<{pr['url']}|#{pr['number']} — {pr['title']}>", people_str]
        if waiting:
            parts.append(waiting)
        lines.append("• " + " — ".join(parts))

    return [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": (
                    f":alarm_clock: *{len(awaiting)} PR(s) waiting on a reviewer*"
                ),
            },
        },
        {
            "type": "section",
            "text": {"type": "mrkdwn", "text": "\n".join(lines)},
        },
    ]


def cmd_remind(args: argparse.Namespace) -> int:
    repo = os.environ["GH_REPO"]
    channel, payload_file = _slack_runtime()
    if not (channel and payload_file):
        log.error(
            "SLACK_CHANNEL and SLACK_PAYLOAD_FILE are required for remind mode"
        )
        return 2

    # Suppress scheduled posts during Warsaw quiet hours (17:00–07:59).
    # The cron is already narrowed to UTC hours that fall inside the
    # working window in both CET and CEST, but DST shifts can drag a
    # tick into the boundary (e.g. UTC 06 in winter = Warsaw 07); the
    # guard here is the precise, DST-aware filter. Manual
    # `workflow_dispatch` runs bypass the check so on-call folks can
    # always poke the bot.
    event_name = os.environ.get("GH_EVENT_NAME", "").strip()
    if event_name == "schedule" and _is_warsaw_quiet_hours():
        log.info(
            "Current Warsaw time is in the quiet window (%d:00–%d:00); "
            "skipping scheduled reminder.",
            QUIET_HOURS_START,
            QUIET_HOURS_END,
        )
        set_gh_output("skip", "true")
        return 0

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
