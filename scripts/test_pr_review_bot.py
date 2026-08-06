"""
Tests for scripts/pr_review_bot.py.

Run with:
    python -m unittest scripts/test_pr_review_bot.py -v

Optional live check against GitHub (requires ``gh`` auth):
    PR_REVIEW_BOT_LIVE=1 python -m unittest scripts/test_pr_review_bot.py -v
"""

from __future__ import annotations

import os
import sys
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
        self.assertEqual(link_line, "<https://github.com/org/repo/pull/1|#1 — No stats on payload>")


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
