#!/usr/bin/env python3
"""Comment on Rust project goal tracking issues when a research post is published.

Meant to run in rust-lang/goals on a schedule, using that repository's own
GITHUB_TOKEN (permissions: issues: write). It reads the research blog's
posts.json and, for each (post, goal) pair, posts one comment on the goal's
tracking issue. A hidden marker in each comment makes it safe to re-run:
existing comments are checked, so no state file is needed.

Usage:
    research_posts.py --feed URL [--repo rust-lang/goals] [--since-days 14] [--dry-run]

Only the Python standard library is used. GITHUB_TOKEN is required unless
--dry-run is given.
"""

import argparse
import datetime
import json
import os
import sys
import urllib.request

MARKER = "<!-- research-post: {url} -->"


def http_json(url, token=None, data=None, method=None):
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("User-Agent", "rust-lang-research-goals-bot")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req) as resp:
        return json.load(resp), resp.headers


def issue_comments(repo, issue, token):
    url = f"https://api.github.com/repos/{repo}/issues/{issue}/comments?per_page=100"
    comments = []
    while url:
        page, headers = http_json(url, token)
        comments.extend(page)
        url = None
        for part in (headers.get("Link") or "").split(","):
            if 'rel="next"' in part:
                url = part[part.index("<") + 1 : part.index(">")]
    return comments


def comment_body(post):
    authors = ", ".join(post["authors"])
    body = f"📝 **New research post:** [{post['title']}]({post['url']}) by {authors}\n"
    if post.get("description"):
        body += f"\n> {post['description']}\n"
    body += "\n" + MARKER.format(url=post["url"]) + "\n"
    return body


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--feed", required=True, help="URL of the blog's posts.json")
    ap.add_argument("--repo", default="rust-lang/goals")
    ap.add_argument(
        "--since-days",
        type=int,
        default=14,
        help="ignore posts older than this, so the first run doesn't backfill everything",
    )
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    token = os.environ.get("GITHUB_TOKEN")
    if not token and not args.dry_run:
        sys.exit("GITHUB_TOKEN is required (or pass --dry-run)")

    feed, _ = http_json(args.feed)
    if feed.get("version") != 1:
        sys.exit(f"unsupported posts.json version: {feed.get('version')}")
    cutoff = datetime.date.today() - datetime.timedelta(days=args.since_days)

    posted = 0
    for post in feed["posts"]:
        if datetime.date.fromisoformat(post["date"]) < cutoff:
            continue
        marker = MARKER.format(url=post["url"])
        for goal in post.get("goals", []):
            issue = goal["issue"]
            existing = issue_comments(args.repo, issue, token)
            if any(marker in (c.get("body") or "") for c in existing):
                print(f"already commented: {args.repo}#{issue} <- {post['url']}")
                continue
            body = comment_body(post)
            if args.dry_run:
                print(f"would comment on {args.repo}#{issue}:\n{body}")
                continue
            http_json(
                f"https://api.github.com/repos/{args.repo}/issues/{issue}/comments",
                token,
                data=json.dumps({"body": body}).encode(),
                method="POST",
            )
            posted += 1
            print(f"commented on {args.repo}#{issue} <- {post['url']}")
    print(f"done; {posted} new comment(s)")


if __name__ == "__main__":
    main()
