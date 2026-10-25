# 0056 — `hql help` prints part of a block body in a signature

Status: todo
Related: [HQL-0038](../proposals/HQL-0038/README.md),
[HQL-0042](../proposals/HQL-0042/README.md),
[`src/help.rs`](../../src/help.rs)

`hql help` on a module whose function has a block body prints the start of
the body inside the signature. With the tutorial vault's
[`club.reading`](../../examples/tutorial/lib/club/reading.hql):

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
   ([`src/reference.rs`](../../src/reference.rs)), for the same defect.
3. A session under `examples/doc-strings/sessions/` showing `hql help` on a
   block-bodied function, and a unit test beside the existing ones.
