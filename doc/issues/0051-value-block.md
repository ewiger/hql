# 0051 — A value block as a function body: bindings, then the value

Status: done
Related: [HQL-0038, functions](../proposals/HQL-0038/README.md),
[the implemented grammar](../models/behavior/expressions.md),
[function examples](../../examples/functions/README.md)

A function body written `{ … }` without `=` was only an iterating block of
`yield`, `for` and bindings. HQL-0038 makes a block equally a value block:
zero or more bindings followed by one final expression, which is the
function's value. Any `fn` takes either body form, at the top level, in a
receiver module, as a trait's default body and in an `impl`.

## Done

- `src/parser.rs`: a function body's block is read line by line. A block
  containing `yield` is iterating; otherwise its last line is an expression,
  and its bindings nest around that expression, each a `match` with one name
  arm, so checking and evaluation need no new form. A trait or `impl`
  function takes a value block.
- Refused at parse: a block that yields and ends in an expression, a block
  that only binds, `for` in a value block, and an expression line before the
  last. `= {` stays a record or set literal.
- A result type mismatch points at the final expression.
- The declared result reaches the final expression through the bindings, as
  any expected type reaches a `match` arm's body, so a lambda on the last
  line takes its parameter types from it.
- Cases under [`examples/functions/`](../../examples/functions/README.md): a
  value block over `Float`, a `Seq<Int>` result without `yield`, one pipeline
  in each body form, block bodies in a trait, an `impl` and a receiver module,
  query steps over a vault, a closure over a local binding, and the four
  refusals.
