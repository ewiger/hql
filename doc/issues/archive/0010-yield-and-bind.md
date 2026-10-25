# 0010 — `yield`: generated sequences as query sources

Status: done

The design is [the queries model](../../models/behavior/queries.md). This issue is
the work that carries it into the grammar, the type system, and the runtime.

A query should start from a sequence produced by an **iterating function** — a
function whose body yields elements and whose result is `Seq<T>`, evaluated only
as far as a consumer demands. `vault` is the current vault, and its field
`cards` is such a sequence. `cards` is nothing more than a binding of that
field, which the standard library makes:

```hql
cards = vault.cards
cards | take(5)
```

`graph = vault.graph` binds the vault's knowledge graph the same way
([HQL-0019](../../proposals/HQL-0019/README.md)).

This depends on [0009](0009-function-type-and-constructor.md): an iterating
function is a user `fn`. It supersedes the seeding done in
[0008](0008-cards-as-a-vault-binding.md): `cards` becomes a lazy `Seq<Card>`
in place of an eager `Set<Card>`, which changes the type of every `cards`
program.

`Stream<T>` lands with this issue, as one feature of
[0014](0014-next-feature-bundle.md) together with steps 2 and 3 of
[0013](../0013-typed-pipeline-operations.md). An iterating function declares
`Seq<T>` or `Stream<T>`, so the type and the first unbounded source arrive
together, over one push traversal carrier.

A vault yields its cards in the order a sorted walk reaches them: the entries
of each directory sorted by name, depth-first, each card yielded as it is
reached. `take(5)` makes that order observable, so it is defined by the walk
and never by the filesystem, and the same query answers the same on every
machine.

Partial application is not part of this issue; it is
[0024](0024-partial-application.md). It is a question for the
function design, not for query sources, as the
[queries model](../../models/behavior/queries.md) states.

## Scope

- Add `yield` to the grammar. A `yield` is written only in the block body of
  a user `fn` — `fn name(...) : Seq<T> { ... }` — directly or inside a
  `for x in xs { ... }` loop. Block bodies and `for` exist only there: HQL has
  no top-level loop, and a lambda body stays one expression.
- Check an iterating function: the presence of `yield` and a `Seq<T>` result
  must agree, and a `yield` outside such a function is a diagnostic.
- Add the lazy sequence carrier to `Value` as a **factory** — how to start a
  fresh traversal — never a live cursor. `Value` derives `Clone`, so a cursor
  would make two uses of one binding answer differently.
- Invert the traversal: `yield` calls a consumer-supplied sink that answers
  whether it wants more, since a tree-walking evaluator cannot suspend its own
  stack at a `yield`. A sink that stops is what makes `take` over an unbounded
  generator terminate.
- Rewrite the eight `elements` call sites so forcing is explicit. That helper is
  the single point where every step materializes its input today, and the lazy
  steps — `map`, `filter`, `take` — wrap the sink instead of calling it.
- Expose the `Vault` trait to HQL as a type whose field `cards` is a lazy
  `Seq<Card>` in walk order, and seed `vault`, the current vault, into the
  checker's and evaluator's scope from the host.
- Define `cards = vault.cards` in the standard library, so no program writes
  the binding itself.
- Audit the extension steps for double traversal and for `size` calls that
  precede a real need. A step that works on a `List` and breaks on a generated
  input is the failure mode this change introduces.

## Done when

- An iterating function can be declared, called, and consumed; a `take` over an
  unbounded generator terminates.
- `cards | take(5)` returns the first five cards of the sorted walk,
  identically on two machines with different filesystem order.
- `cards` is the library binding `cards = vault.cards`, not a keyword and not
  a special case in the checker or the evaluator.
- `cards` is a `Seq<Card>`, and the existing `cards` examples check and pass
  at that type.
- Examples under `examples/` cover a generated source, early termination over
  a long generator, and `invalid` cases for
  `yield` at the top level and inside a lambda — all passing through
  `tests/corpus.rs`.
- No extension step traverses a sequence twice without materializing it first.
