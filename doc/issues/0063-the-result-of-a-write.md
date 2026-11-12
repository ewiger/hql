# 0063 — The result of a write: `Unit` is the wrong answer

Status: todo
Related: [HQL-0021](../proposals/HQL-0021/README.md) (`target` and `sink`),
[HQL-0045](../proposals/HQL-0045/README.md) (the error model and the
diagnostic channel), [HQL-0037](../proposals/HQL-0037/README.md) (`fork`,
whose value is the record of its branches),
[`lib/std/io/mod.hql`](../../lib/std/io/mod.hql),
[writing with `target` and `sink`](../public/learn/09-io/target.md),
[deduplication, in the load](../public/learn/20-olap/deduplication.md#in-the-load)

`target` and `sink` give `Unit`, the built-in type with one value, and a
checked pipeline that ends in a write therefore prints `Unit`. The name is
the problem, and so is the type behind it. A reader meeting `Unit` on a book
page cannot tell whether it is a type, a value, a missing value or a status;
it reads as `none` with a capital letter, and the book has to explain it
away at every write. The type is the deeper problem: a write is the one
step whose outcome a pipeline most wants to know, and the step gives back
the one value that says nothing.

## What a write could give

A write has an outcome, and the outcome is data:

- **What was written.** The number of rows or documents written, and where:
  the endpoint by its id, the table or subject, the commit or the offset
  where the system reports one. A `sink` over a stream has such a count at
  every moment, which is what a progress report and a run's summary read.
- **Whether it succeeded.** A failed write is an evaluation failure today
  and stops the program, by the error model. A pipeline that must continue,
  as the dead-letter branches of chapters 16 and 17 do for reads, has no
  way to route a failed write. The outcome of a write may belong in the
  same place a failed decode goes: a `Result`, or the run's diagnostic
  channel, with the write going on.
- **Nothing, by name.** Where the answer stays "nothing", the name must say
  so: `Done`, `Written`, `Nothing`, `Void`. Each reads as a verdict rather
  than as a mathematical word.

The `fork` record of chapter 19, `{silver: (), rejected: ()}`, shows the
value as well as the type, and in a different spelling, `()` beside `empty`
in a record. Whatever the type becomes, its one spelling is part of this.

## Scope

- The name, if the type stays a one-valued type.
- The type of `target` and `sink`, if a write has an outcome worth
  returning, and what a `Stream`'s `sink` gives while the stream runs.
- Where a failed write goes under a policy that continues.
- One printed form for the value.
- The book and reference pages that say `Unit`, and the `hql check` output
  the book shows.

## Done when

- A program that ends in a write checks to a type whose name says what the
  program gives.
- The value prints one way, everywhere.
- A write's outcome, if it has one, is a value a later step can read, with
  the count and the endpoint in it.
- Every `Unit` in `lib/std`, the book and the reference is the new name or
  the new type, and `CHANGELOG.md` lists the change under breaking changes.
