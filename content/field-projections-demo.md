+++
title = "Demo: Sketching field projections through places"
date = 2026-11-05
authors = ["Ferris (demo author)"]
description = "A placeholder post showing goal links, tags, code highlighting, and footnotes."
[taxonomies]
tags = ["field-projections", "places"]
[extra]
goals = ["field-projections"]
discussion = "https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang"
+++

*This is placeholder content for the demo. It doesn't describe the real
field projections design.*

A research post usually starts with the problem it is about:

```rust
struct Pair<T> {
    left: T,
    right: T,
}

fn project<T>(pair: Pin<&mut Pair<T>>) -> Pin<&mut T> {
    // Today this needs unsafe code or a macro.
    todo!()
}
```

Then it sketches a direction, without having to be complete or final.[^axioms]

## Open questions

- How should projections interact with `Drop`?
- What would the syntax be?

[^axioms]: "Embrace imperfection" is one of the blog's design axioms.
