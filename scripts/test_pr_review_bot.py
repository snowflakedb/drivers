"""
Tests for scripts/pr_review_bot.py.

Run with:
    python -m unittest scripts/test_pr_review_bot.py -v

Optional live check against GitHub (requires ``gh`` auth):
    PR_REVIEW_BOT_LIVE=1 python -m unittest scripts/test_pr_review_bot.py -v
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).parent))
import pr_review_bot as bot  # noqa: E402


class FormatPrChangeStatsTests(unittest.TestCase):
    def test_formats_additions_and_deletions(self) -> None:
        self.assertEqual(
            bot._format_pr_change_stats({"additions": 175, "deletions": 0}),
            "+175/-0",
        )
        self.assertEqual(
            bot._format_pr_change_stats({"additions": 10, "deletions": 3}),
            "+10/-3",
        )

    def test_treats_missing_values_as_zero(self) -> None:
        self.assertEqual(
            bot._format_pr_change_stats({"additions": None, "deletions": 2}),
            "+0/-2",
        )
        self.assertEqual(
            bot._format_pr_change_stats({"additions": 5, "deletions": None}),
            "+5/-0",
        )

    def test_returns_empty_when_stats_absent(self) -> None:
        self.assertEqual(bot._format_pr_change_stats({}), "")
        self.assertEqual(
            bot._format_pr_change_stats({"additions": None, "deletions": None}),
            "",
        )


class _NeverOoo:
    """Stub Slack-status lookup used by assign-pool tests."""

    def is_ooo(self, login: str) -> bool:
        return False


class PickAssignReviewerTests(unittest.TestCase):
    def test_uses_label_pool_when_someone_other_than_the_author_is_eligible(
        self,
    ) -> None:
        picked = bot._pick_assign_reviewer(
            candidates=["alice_snow", "bob_snow"],
            full_pool=["alice_snow", "bob_snow", "carol_snow"],
            excluded=["alice_snow"],
            names=_NeverOoo(),
        )
        self.assertEqual(picked, "bob_snow")

    def test_falls_back_to_full_pool_when_label_pool_is_only_the_author(
        self,
    ) -> None:
        # PR #1558: transport rule is a one-person pool matching the author.
        picked = bot._pick_assign_reviewer(
            candidates=["bartosz-oler_snow"],
            full_pool=["bartosz-oler_snow", "alice_snow", "bob_snow"],
            excluded=["bartosz-oler_snow"],
            names=_NeverOoo(),
        )
        self.assertIn(picked, {"alice_snow", "bob_snow"})

    def test_does_not_assign_the_author_from_the_fallback_pool(self) -> None:
        picked = bot._pick_assign_reviewer(
            candidates=["bartosz-oler_snow"],
            full_pool=["bartosz-oler_snow", "alice_snow"],
            excluded=["bartosz-oler_snow"],
            names=_NeverOoo(),
        )
        self.assertEqual(picked, "alice_snow")

    def test_returns_none_when_full_pool_is_only_the_author(self) -> None:
        picked = bot._pick_assign_reviewer(
            candidates=["bartosz-oler_snow"],
            full_pool=["bartosz-oler_snow"],
            excluded=["bartosz-oler_snow"],
            names=_NeverOoo(),
        )
        self.assertIsNone(picked)

    def test_does_not_retry_when_candidates_already_are_the_full_pool(
        self,
    ) -> None:
        roster = ["bartosz-oler_snow"]
        picked = bot._pick_assign_reviewer(
            candidates=roster,
            full_pool=roster,
            excluded=["bartosz-oler_snow"],
            names=_NeverOoo(),
        )
        self.assertIsNone(picked)


class AssignSlackMessageTests(unittest.TestCase):
    def test_link_line_includes_backtick_stats_suffix(self) -> None:
        pr = {
            "additions": 175,
            "deletions": 0,
            "title": "SNOW-3784525: Improve ud-no-unwrap-in-production",
            "html_url": "https://github.com/org/repo/pull/597",
        }
        pr_number = 597
        title = pr["title"]
        html_url = pr["html_url"]

        change_stats = bot._format_pr_change_stats(pr)
        stats_suffix = f" `{change_stats}`" if change_stats else ""
        link_line = f"<{html_url}|#{pr_number} — {title}>{stats_suffix}"

        self.assertEqual(change_stats, "+175/-0")
        self.assertEqual(
            link_line,
            "<https://github.com/org/repo/pull/597|#597 — SNOW-3784525: "
            "Improve ud-no-unwrap-in-production> `+175/-0`",
        )

    def test_omits_suffix_when_stats_missing(self) -> None:
        pr = {
            "title": "No stats on payload",
            "html_url": "https://github.com/org/repo/pull/1",
        }
        change_stats = bot._format_pr_change_stats(pr)
        stats_suffix = f" `{change_stats}`" if change_stats else ""
        link_line = (
            f"<{pr['html_url']}|#1 — {pr['title']}>{stats_suffix}"
        )
        self.assertEqual(
            link_line,
            "<https://github.com/org/repo/pull/1|#1 — No stats on payload>",
        )


class _FakeNames:
    def name(self, login: str) -> str:
        return f"@{login}"

    def is_ooo(self, login: str) -> bool:
        return False

    def ooo_emoji(self, login: str) -> str | None:
        return None


def _awaiting_pr(number: int, title: str, *, waiting_hours: float = 5.0) -> dict:
    return {
        "number": number,
        "title": title,
        "url": f"https://github.com/org/repo/pull/{number}",
        "requested": ["alice"],
        "waiting_hours": waiting_hours,
        "waiting_source": "review_requested",
    }


class SlackMrkdwnEscapeTests(unittest.TestCase):
    def test_escapes_broadcast_and_link_metacharacters(self) -> None:
        self.assertEqual(
            bot._escape_slack_mrkdwn("<!channel> & more | extra"),
            "&lt;!channel&gt; &amp; more / extra",
        )

    def test_reminder_line_does_not_keep_raw_channel_ping(self) -> None:
        names = _FakeNames()
        line = bot._format_reminder_line(
            _awaiting_pr(9, "<!channel> pwn"),
            names,
        )
        self.assertNotIn("<!channel>", line)
        self.assertIn("&lt;!channel&gt;", line)
        self.assertIn(
            "<https://github.com/org/repo/pull/9|#9 — &lt;!channel&gt; pwn>",
            line,
        )

    def test_escapes_href_broadcast_markup(self) -> None:
        names = _FakeNames()
        pr = _awaiting_pr(9, "ok")
        pr["url"] = "https://evil.example/<!channel>"
        line = bot._format_reminder_line(pr, names)
        self.assertNotIn("<!channel>", line)
        self.assertIn(
            "<https://evil.example/&lt;!channel&gt;|#9 — ok>",
            line,
        )

    def test_decorate_reviewer_escapes_plain_text_and_untrusted_emoji(
        self,
    ) -> None:
        class _HostileNames:
            def name(self, login: str) -> str:
                return "<!channel>"

            def is_ooo(self, login: str) -> bool:
                return True

            def ooo_emoji(self, login: str) -> str | None:
                return "<!here>"

        rendered = bot._decorate_reviewer(_HostileNames(), "alice")
        self.assertEqual(rendered, "&lt;!channel&gt; :zzz:")

    def test_decorate_reviewer_keeps_real_slack_mentions(self) -> None:
        class _MentionNames:
            def name(self, login: str) -> str:
                return "<@U123ABC>"

            def is_ooo(self, login: str) -> bool:
                return False

            def ooo_emoji(self, login: str) -> str | None:
                return None

        self.assertEqual(
            bot._decorate_reviewer(_MentionNames(), "alice"),
            "<@U123ABC>",
        )


def _open_pr(
    *,
    number: int = 1797,
    author: str = "alice",
    requested: tuple[str, ...] = ("bob",),
    title: str = "Raise security-signoff label recall",
) -> dict:
    return {
        "number": number,
        "title": title,
        "html_url": f"https://github.com/org/repo/pull/{number}",
        "user": {"login": author},
        "requested_reviewers": [
            {"login": login, "type": "User"} for login in requested
        ],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }


def _review(login: str, state: str, *, bot: bool = False) -> dict:
    user = {
        "login": f"{login}[bot]" if bot and not login.endswith("[bot]") else login,
        "type": "Bot" if bot else "User",
    }
    return {"user": user, "state": state}


class ClassifyPrForReminderTests(unittest.TestCase):
    _now = datetime(2026, 9, 9, 9, 0, tzinfo=timezone.utc)
    _requested_at = _now - timedelta(hours=5)

    def _classify(self, pr: dict, reviews: list[dict]) -> dict | None:
        return bot._classify_pr_for_reminder(
            pr, reviews, first_request_time=self._requested_at, now=self._now
        )

    def test_keeps_pr_when_only_the_author_left_a_comment_review(self) -> None:
        entry = self._classify(
            _open_pr(author="alice", requested=("bob",)),
            [_review("alice", "COMMENTED")],
        )
        self.assertIsNotNone(entry)
        self.assertEqual(entry["number"], 1797)
        self.assertEqual(entry["requested"], ["bob"])

    def test_drops_pr_when_a_reviewer_commented(self) -> None:
        self.assertIsNone(
            self._classify(
                _open_pr(author="alice", requested=("bob",)),
                [_review("bob", "COMMENTED")],
            )
        )

    def test_keeps_pr_when_only_bots_commented(self) -> None:
        entry = self._classify(
            _open_pr(author="alice", requested=("bob",)),
            [
                _review("snowflake-security-bot-internal", "COMMENTED", bot=True),
                _review("ai-review-bot-1", "COMMENTED", bot=True),
            ],
        )
        self.assertIsNotNone(entry)
        self.assertEqual(entry["requested"], ["bob"])

    def test_drops_pr_when_a_reviewer_approved_even_if_author_also_commented(
        self,
    ) -> None:
        self.assertIsNone(
            self._classify(
                _open_pr(author="alice", requested=("bob",)),
                [
                    _review("alice", "COMMENTED"),
                    _review("bob", "APPROVED"),
                ],
            )
        )


class ReminderDigestChunkTests(unittest.TestCase):
    def test_chunk_empty(self) -> None:
        self.assertEqual(bot._chunk_text_lines([]), [])

    def test_single_short_line_is_one_chunk(self) -> None:
        self.assertEqual(bot._chunk_text_lines(["hello"]), [["hello"]])

    def test_splits_when_joined_text_exceeds_limit(self) -> None:
        lines = ["aaa", "bbb", "ccc"]
        chunks = bot._chunk_text_lines(lines, limit=7)
        # "aaa\nbbb" is 7 chars; next line starts a new chunk.
        self.assertEqual(chunks, [["aaa", "bbb"], ["ccc"]])
        for chunk in chunks:
            self.assertLessEqual(len("\n".join(chunk)), 7)

    def test_truncates_a_line_longer_than_the_limit(self) -> None:
        line = "x" * 50
        chunks = bot._chunk_text_lines([line], limit=10)
        self.assertEqual(len(chunks), 1)
        self.assertEqual(len(chunks[0][0]), 10)
        self.assertTrue(chunks[0][0].endswith("…"))

    def test_reminder_messages_fit_slack_section_limit(self) -> None:
        # ~40 PRs with long titles used to blow the 3000-char section cap
        # and Slack dropped the whole digest.
        names = _FakeNames()
        awaiting = [
            _awaiting_pr(
                i,
                f"SNOW-{4000000 + i}: {'very long title ' * 8}{i}",
            )
            for i in range(1, 41)
        ]
        messages = bot._reminder_messages(awaiting, names)
        self.assertGreater(len(messages), 1)
        for fallback, blocks in messages:
            self.assertIn("waiting on a reviewer", fallback)
            self.assertEqual(len(blocks), 2)
            heading = blocks[0]["text"]["text"]
            body = blocks[1]["text"]["text"]
            self.assertLessEqual(len(heading), bot.SLACK_MRKDWN_TEXT_LIMIT)
            self.assertLessEqual(len(body), bot.SLACK_MRKDWN_TEXT_LIMIT)
            self.assertIn("/", heading)  # "(1/n)"
        self.assertIn("(1/", messages[0][1][0]["text"]["text"])
        self.assertTrue(messages[-1][1][0]["text"]["text"].endswith(
            f"({len(messages)}/{len(messages)})"
        ))

    def test_single_message_omits_part_index(self) -> None:
        names = _FakeNames()
        messages = bot._reminder_messages(
            [_awaiting_pr(1, "tiny")],
            names,
        )
        self.assertEqual(len(messages), 1)
        heading = messages[0][1][0]["text"]["text"]
        self.assertEqual(
            heading,
            ":alarm_clock: *1 PR(s) waiting on a reviewer*",
        )
        self.assertNotIn("/", heading)

    def test_write_reminder_payloads_numbers_files(self) -> None:
        names = _FakeNames()
        awaiting = [
            _awaiting_pr(i, f"title {'z' * 200} {i}")
            for i in range(1, 30)
        ]
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            n = bot.write_reminder_payloads(
                directory, "drivers-review", awaiting, names
            )
            files = sorted(directory.glob("*.json"))
            self.assertEqual(n, [f.name for f in files])
            self.assertGreaterEqual(len(n), 2)
            self.assertEqual(files[0].name, "00.json")
            first = json.loads(files[0].read_text())
            self.assertEqual(first["channel"], "drivers-review")
            self.assertTrue(first["blocks"])


class _StubDisplay:
    """Offline stand-in for ``ReviewerDisplay``: deterministic mention,
    never OOO, no GitHub/Slack calls."""

    def __init__(self, repo: str | None = None, slack_token: str | None = None) -> None:
        pass

    def name(self, login: str) -> str:
        return f"<@{login}>"

    def is_ooo(self, login: str) -> bool:
        return False


# A minimal pool: one plain reviewer and one with the ``notify: false``
# opt-out, plus a couple of rules so the random-pick fallback has a
# non-empty roster.
_REVIEWERS_YAML = """\
reviewers:
  alice_snow:
  bob_snow:
    notify: false
rules:
  all:
    - alice_snow
  python:
    - alice_snow
    - bob_snow
"""


class AssignReuseExistingReviewerTests(unittest.TestCase):
    """When a PR already carries a human reviewer, ``cmd_assign`` reuses
    that reviewer instead of skipping: it reconciles the GitHub
    review-request + assignee for them (``gh_pr_assign`` is idempotent and
    sets both surfaces) and still posts the channel announcement, subject
    to the reused reviewer's ``notify`` preference."""

    def _pr(
        self,
        requested: list[dict],
        *,
        author: str = "dave_snow",
        labels: list[str] | None = None,
    ) -> dict:
        return {
            "number": 42,
            "draft": False,
            "state": "open",
            "user": {"login": author},
            "title": "Add thing",
            "html_url": "https://github.com/snowflakedb/drivers/pull/42",
            "additions": 10,
            "deletions": 2,
            "labels": [{"name": name} for name in (labels or [])],
            "requested_reviewers": requested,
        }

    def _run_assign(self, pr: dict) -> tuple[list[str], dict | None]:
        """Run ``cmd_assign`` against *pr* with every network seam stubbed.

        Returns ``(assign_calls, payload)`` where ``assign_calls`` is the
        list of logins passed to ``gh_pr_assign`` and ``payload`` is the
        parsed Slack payload, or ``None`` when no channel post was written.
        """
        assign_calls: list[str] = []
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            reviewers = root / "reviewers.yml"
            reviewers.write_text(_REVIEWERS_YAML)
            payload_file = root / "payload.json"
            env = {
                "GH_REPO": "snowflakedb/drivers",
                "PR_NUMBER": str(pr["number"]),
                "REVIEWERS_PATH": str(reviewers),
                "SLACK_CHANNEL": "drivers-review",
                "SLACK_PAYLOAD_FILE": str(payload_file),
                # Keep set_gh_output a no-op regardless of the outer env.
                "GITHUB_OUTPUT": "",
            }
            with mock.patch.dict(os.environ, env, clear=False), mock.patch.object(
                bot, "get_pr", lambda repo, n: pr
            ), mock.patch.object(
                bot, "ReviewerDisplay", _StubDisplay
            ), mock.patch.object(
                bot,
                "gh_pr_assign",
                lambda repo, n, login: assign_calls.append(login),
            ), mock.patch.object(
                bot, "gh_pr_remove_reviewer", lambda repo, n, login: None
            ):
                rc = bot.cmd_assign(argparse.Namespace())
            payload = (
                json.loads(payload_file.read_text())
                if payload_file.exists()
                else None
            )
        self.assertEqual(rc, 0)
        return assign_calls, payload

    def test_reuses_existing_reviewer_and_posts_slack(self) -> None:
        # carol_snow is a human reviewer already on the PR (and not even in
        # the pool). The bot reuses her rather than picking someone new,
        # and announces the PR naming her.
        assign_calls, payload = self._run_assign(
            self._pr([{"login": "carol_snow", "type": "User"}])
        )
        self.assertEqual(assign_calls, ["carol_snow"])
        self.assertIsNotNone(payload)
        blob = json.dumps(payload)
        self.assertIn("carol_snow", blob)
        self.assertIn("New PR ready for review", blob)

    def test_reuses_only_the_first_when_multiple_requested(self) -> None:
        assign_calls, _ = self._run_assign(
            self._pr(
                [
                    {"login": "carol_snow", "type": "User"},
                    {"login": "erin_snow", "type": "User"},
                ]
            )
        )
        # Only the first requested reviewer is reconciled; the rest keep
        # their existing GitHub review request and are left untouched.
        self.assertEqual(assign_calls, ["carol_snow"])

    def test_reuse_honors_notify_false(self) -> None:
        # bob_snow carries notify:false. The GitHub side is still
        # reconciled, but the channel post is suppressed (Option A).
        assign_calls, payload = self._run_assign(
            self._pr([{"login": "bob_snow", "type": "User"}])
        )
        self.assertEqual(assign_calls, ["bob_snow"])
        self.assertIsNone(payload)

    def test_author_and_bot_reviewers_do_not_count_as_existing(self) -> None:
        # Only the PR author (stripped) and a bot reviewer are present, so
        # there is no human to reuse: the bot falls through to its normal
        # random pick from the pool and never reuses the author or the bot.
        assign_calls, _ = self._run_assign(
            self._pr(
                [
                    {"login": "dave_snow", "type": "User"},
                    {"login": "copilot[bot]", "type": "Bot"},
                ]
            )
        )
        self.assertEqual(len(assign_calls), 1)
        self.assertIn(assign_calls[0], {"alice_snow", "bob_snow"})
        self.assertNotIn(assign_calls[0], {"dave_snow", "copilot[bot]"})


@unittest.skipUnless(
    os.environ.get("PR_REVIEW_BOT_LIVE"),
    "set PR_REVIEW_BOT_LIVE=1 to run live GitHub checks",
)
class LiveGitHubPrStatsTests(unittest.TestCase):
    def test_get_pr_includes_change_stats(self) -> None:
        repo = os.environ.get("GH_REPO", "snowflakedb/drivers")
        pr_number = int(os.environ.get("PR_NUMBER", "597"))
        pr = bot.get_pr(repo, pr_number)

        self.assertIsInstance(pr.get("additions"), int)
        self.assertIsInstance(pr.get("deletions"), int)
        self.assertTrue(bot._format_pr_change_stats(pr))


if __name__ == "__main__":
    unittest.main()
