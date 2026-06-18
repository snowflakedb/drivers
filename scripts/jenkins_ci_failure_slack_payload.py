#!/usr/bin/env python3
"""Build a Slack payload for Jenkins CI failures.

This mirrors the Block Kit structure used by the GitHub Actions
`notify-slack-failure.yml` workflow, but reads inputs from Jenkins.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET


def _resolve_author_mention(
    *,
    author_email: str,
    author_name: str,
    slack_bot_token: str,
) -> str:
    """Resolve a Slack mention via users.lookupByEmail.

    Tags the matched Slack user (``<@id>``); on any miss or lookup failure
    falls back to the author name, then the email, then ``unknown`` — matching
    the GitHub Actions notifier.
    """
    fallback = author_name or author_email or "unknown"
    if not author_email or not slack_bot_token:
        return fallback

    endpoint = (
        "https://slack.com/api/users.lookupByEmail?email="
        + urllib.parse.quote(author_email)
    )
    req = urllib.request.Request(
        endpoint,
        headers={"Authorization": f"Bearer {slack_bot_token}"},
        method="GET",
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError):
        return fallback

    if data.get("ok") and (data.get("user") or {}).get("id"):
        return f"<@{data['user']['id']}>"
    return fallback


def _collect_test_failures(artifacts_root: str) -> list[tuple[str, str]]:
    """Parse JUnit XML files and return (label, message) failures."""
    failures: list[tuple[str, str]] = []
    pattern = os.path.join(artifacts_root, "**", "*.xml")
    for xml_path in sorted(glob.glob(pattern, recursive=True)):
        try:
            tree = ET.parse(xml_path)
        except Exception:
            continue
        for testcase in tree.findall(".//testcase"):
            for tag in ("failure", "error"):
                node = testcase.find(tag)
                if node is None:
                    continue
                classname = testcase.get("classname", "")
                name = testcase.get("name", "")
                msg = (node.get("message") or node.text or "").strip()[:200]
                label = f"{classname}.{name}" if classname else name
                failures.append((label, msg))
                break
    return failures


def _build_payload(args: argparse.Namespace) -> dict:
    sha = (args.head_sha or "")[:8]
    event = args.trigger_event
    if event == "merge_group":
        context_str = "merge queue \u2192 `main`"
    else:
        context_str = f"push to `{args.head_branch}`"

    failed_jobs = json.loads(args.failed_jobs_json or "[]")
    test_failures = _collect_test_failures(args.artifacts_root)
    author_mention = _resolve_author_mention(
        author_email=args.author_email or "",
        author_name=args.author_name or "",
        slack_bot_token=os.environ.get("SLACK_BOT_TOKEN", ""),
    )

    blocks: list[dict] = [
        {
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": f"\u274c CI Failure: {args.workflow_name}",
                "emoji": True,
            },
        },
        {
            "type": "section",
            "fields": [
                {"type": "mrkdwn", "text": f"*Trigger:*\n{context_str}"},
                {"type": "mrkdwn", "text": f"*Commit:*\n`{sha}`"},
                {"type": "mrkdwn", "text": f"*Author:*\n{author_mention}"},
            ],
        },
    ]

    if failed_jobs:
        lines = []
        for job in failed_jobs[:10]:
            step = f" · step: _{job['failed_step']}_" if job.get("failed_step") else ""
            lines.append(f"• <{job['url']}|{job['name']}>{step}")
        if len(failed_jobs) > 10:
            lines.append(f"_...and {len(failed_jobs) - 10} more_")
        blocks.append(
            {
                "type": "section",
                "text": {"type": "mrkdwn", "text": "*Failed jobs:*\n" + "\n".join(lines)},
            }
        )

    if test_failures:
        cap = 15
        lines = [
            f"• `{label}`: {msg}" if msg else f"• `{label}`"
            for label, msg in test_failures[:cap]
        ]
        suffix = (
            f"\n_...and {len(test_failures) - cap} more_"
            if len(test_failures) > cap
            else ""
        )
        blocks.append(
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "*Failed tests:*\n" + "\n".join(lines) + suffix,
                },
            }
        )

    blocks.append(
        {
            "type": "actions",
            "elements": [
                {
                    "type": "button",
                    "text": {"type": "plain_text", "text": "View Run"},
                    "url": args.run_url,
                    "style": "danger",
                }
            ],
        }
    )

    return {
        "channel": args.slack_channel,
        "blocks": blocks,
        "unfurl_links": False,
        "unfurl_media": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workflow-name", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--head-branch", required=True)
    parser.add_argument("--trigger-event", required=True)
    parser.add_argument("--failed-jobs-json", required=True)
    parser.add_argument("--author-email", default="")
    parser.add_argument("--author-name", default="")
    parser.add_argument("--slack-channel", required=True)
    parser.add_argument("--artifacts-root", required=True)
    parser.add_argument("--payload-file", required=True)
    args = parser.parse_args()

    payload = _build_payload(args)
    os.makedirs(os.path.dirname(args.payload_file), exist_ok=True)
    with open(args.payload_file, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)

    failed_jobs_count = len(json.loads(args.failed_jobs_json or "[]"))
    test_failures_count = len(_collect_test_failures(args.artifacts_root))
    print(
        "Payload: "
        f"{failed_jobs_count} failed job(s), {test_failures_count} failed test(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
