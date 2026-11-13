# 0056 — `hql help` prints part of a block body in a signature

Status: done
Related: [HQL-0038](../../proposals/HQL-0038/README.md),
[HQL-0042](../../proposals/HQL-0042/README.md),
[`src/help.rs`](../../../src/help.rs)

`hql help` on a module whose function has a block body prints the start of
the body inside the signature. With the tutorial vault's
[`club.reading`](../../../examples/tutorial/lib/club/reading.hql):

```sh
hql --vault examples/tutorial help club.reading
```

```text
fn days(pages: Int, per_day: Int) : Int
    Days to read `pages` at `per_day` pages a day; a part day counts as one.
fn days_for_all(shelf: List<Int>, per_day: Int) : Int { each = shelf | map(pages => days(pages, per_day))
    Days to read every book of `shelf`, one after another.
```

`days_for_all` should print as `fn days_for_all(shelf: List<Int>, per_day:
Int) : Int`. A function with an `=` body prints correctly.

## Work

1. `src/help.rs`: a declaration's head ends before the block's opening `{`
   (`head_end`, `one_line`), wherever the function is declared — a program,
   a module, a receiver module, a trait default body and an `impl`. A `{` in
   a parameter's or result's type, `= {` record literals included, is not a
   block.
2. Check `hql reference`, which renders signatures from the same source
   ([`src/reference.rs`](../../../src/reference.rs)), for the same defect.
3. A session under `examples/doc-strings/sessions/` showing `hql help` on a
   block-bodied function, and a unit test beside the existing ones.

## Outcome

A declaration's head now ends where the parser says it does, not where its
body begins: a function's and a trait function's at the end of the result
type, an alias's at the `)` of its parameters, or at its name when it has
none, which the parser records on the alias. A body's span starts inside the
block's `{` and after a leading `(`, so both had leaked into the signature;
an `=` body opening with `(` printed as `fn up(n: Int, d: Int) : Int = (`.
The `=` and `:=` that `one_line` used to strip are no longer reached.

`hql reference` renders from the same declarations; regenerating it changes
nothing. Unit tests in [`src/help.rs`](../../../src/help.rs) cover a block
body, an expression body, an alias and trait functions; the session under
`examples/doc-strings/sessions/` was not added.
