"""
Tests for scripts/pr_review_bot.py.

Run with:
    python -m unittest scripts/test_pr_review_bot.py -v

Optional live check against GitHub (requires ``gh`` auth):
    PR_REVIEW_BOT_LIVE=1 python -m unittest scripts/test_pr_review_bot.py -v
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

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
