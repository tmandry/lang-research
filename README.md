# Rust Lang Research

Research notes from people working on the design of the Rust language.
Members of the Rust Project use this blog to share ideas that are still taking
shape. Its main audience is the Project itself.

The site is built with [Zola](https://www.getzola.org/) (v0.23.6). Its
templates are derived from [blog.rust-lang.org](https://github.com/rust-lang/blog.rust-lang.org);
see [UPSTREAM.md](UPSTREAM.md).

## Building

```sh
zola serve --open
```

If you use [mise](https://mise.jdx.dev), `mise run serve` uses the pinned Zola version.

## Writing a post

Add a Markdown file under `content/`:

```toml
+++
title = "My idea"
date = 2026-11-01
authors = ["Your Name"]
description = "A one or two sentence summary."
[taxonomies]
tags = ["some-topic"]
+++

Your post here.
```

Anyone with write access to this repository may merge their own post once CI passes.

## License

Like the main Rust blog, this content is licensed under either of
[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. By submitting
a post you agree to license it under these terms.
