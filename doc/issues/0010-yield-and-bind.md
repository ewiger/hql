# 0010 — `yield` and `bind`: generated sequences as query sources

Status: backlog

The design is [the queries model](../models/behavior/queries.md). This issue is
the work that carries it into the grammar, the type system, and the runtime.

A query should start from a sequence produced by an **iterating function** — a
function whose body yields elements and whose result is `Seq<T>`, evaluated only
as far as a consumer demands — and `cards` should be nothing more than one such
function partially applied to the vault:

```hql
cards = bind(get_all_cards, vault)
cards | take(5)
```

This depends on [0009](0009-function-type-and-constructor.md): without a
function type there is nothing for `bind` to take or return. It supersedes the
seeding proposed in [0008](0008-cards-as-a-vault-binding.md) — `cards` becomes a
lazy `Seq<Card>` rather than an eager `Set<Card>`.

## Decide before building

Two questions in the model have to be settled first, because the answers change
the type system rather than the code around it.

**Whether `Stream` lands in this issue.** Laziness alone needs no new type: a
restartable lazy sequence is observationally a `Seq`, so every existing step
keeps working. `Stream<T>` is what expresses a source with *no finite size*, and
no such source exists yet. The model recommends shipping lazy `Seq` here and
adding `Stream<T>`, with `Seq<T> : {Stream<T>, Collection<T>}`, when something
is genuinely unbounded.

**The order a vault yields in.** `cards` is a `Set<Card>` today precisely so
that discovery order stays unobservable. `take(5)` makes it observable, so the
generator must yield in a defined order — by card name — or the same query
answers differently on two machines. This is a correctness requirement, not a
preference.

**The name `bind`.** It already means binding a type parameter in
[generic brackets](../models/behavior/bind-vs-apply-in-generic-types.md). One
of the two uses should move.

## Scope

- Add `yield` to the grammar, and the loop or comprehension form it appears
  inside — HQL has no statement-level loop today, which is an open question in
  the model rather than a detail.
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
- Add `bind` as partial application over a function value, checked against the
  function type from 0009.
- Expose the `Vault` trait to HQL as a type, with `get_all_cards` as its first
  iterating function, and seed `vault` into the checker's and evaluator's scope
  from the host.
- Define `cards` in the standard library as the bound source, so no program
  writes the binding itself.
- Audit the extension steps for double traversal and for `size` calls that
  precede a real need. A step that works on a `List` and breaks on a generated
  input is the failure mode this change introduces.

## Done when

- An iterating function can be declared, called, and consumed; a `take` over an
  unbounded generator terminates.
- `cards | take(5)` returns the first five cards by name, identically on two
  machines with different filesystem order.
- `cards` is a library binding, not a keyword and not a special case in the
  checker or the evaluator.
- `bind` type-checks against arity and parameter types, and rejects a mismatch
  with a diagnostic naming the parameter.
- Examples under `examples/` cover a generated source, a partially applied
  step, early termination over a long generator, and an `invalid` case for
  `yield` outside an iterating function — all passing through
  `tests/corpus.rs`.
- No extension step traverses a sequence twice without materializing it first.
