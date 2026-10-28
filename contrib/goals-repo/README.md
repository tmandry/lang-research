# Project goals integration (for rust-lang/goals)

These files are meant to be proposed to
[rust-lang/goals](https://github.com/rust-lang/goals). They are kept here so the
demo is self-contained.

`research_posts.py` reads this blog's `posts.json` and comments on the tracking
issue of each goal a new post mentions:

> 📝 **New research post:** [Title](https://…) by Author
>
> > The post's description.

Why this design:

- **No new credentials.** The workflow runs in rust-lang/goals and uses that
  repository's own `GITHUB_TOKEN` with `issues: write`. That repository already
  runs a scheduled workflow with the same kind of token. No GitHub App, bot account, or
  cross-repository secret is needed, so infra doesn't need to change anything.
- **Monthly goal updates pick these up.** `rust-project-goals-cli updates`
  includes every tracking-issue comment except a few automated ones identified by
  prefix, so research posts show up in the monthly goal updates.
- **No state to store.** Each comment has a hidden
  `<!-- research-post: URL -->` marker, and existing comments are checked first.
- **Doesn't backfill.** Posts older than `--since-days` (default 14) are skipped.

Try it without posting anything:

```sh
python3 research_posts.py --feed https://lang-research.rust-lang.org/posts.json --dry-run
# or against a local build:
zola serve &   # then
python3 research_posts.py --feed http://127.0.0.1:1111/posts.json --dry-run
```
