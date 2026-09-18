# What implementing the CLI settled

Recorded 2026-09-18. The user asked for the binary built out to the vault, with
rich diagnostics, `run`, stdin, a REPL and JSON — and accepted that implementing
undecided questions settles them, so each one is recorded rather than left
implicit.

Decided in passing, all small:

- **Generic brackets are `[]`** — `Set[Card]`, not `Set<Card>`. The newest model
  documents already used them, and `<` stops working as a bracket once
  comparison operators exist.
- **Parentheses group** and **`//` begins a comment**, both of which
  [expressions](../models/behavior/expressions.md) previously ruled out.
- **`==` and `!=` exist**, comparing scalars, binding looser than `+`, and not
  chaining. Either side may be a `Data` leaf, which is what asking a header a
  question looks like. They were added because the to-do-list use case — cards
  with a given status — cannot be written without them.
- **An extension contributes under a key it owns**: `metadata.search.model`,
  `metadata.git.*`. Authored and contributed entries then cannot collide, which
  sidesteps `CON-07` for metadata rather than answering it. Derived header
  paths — `name`, `path`, `format` — still win over anything authored, which is
  the part of `CON-07` this does answer.
- **A relation card's kind is checked at load.** Endpoints that do not resolve
  mean the claim fails, the card stays a `ConceptCard`, and the failed claim is
  kept at `metadata.knowledge.unverified` with a warning. Nothing is trusted and
  nothing is lost — the user's choice.
- **A relation card contributes an edge and does not become a node**, and its
  endpoints enter the graph as expanded nodes so the edge has ends.
- **`expand` and `graph` are two stages.** `expand(depth = n)` traverses and
  produces a graph carrying why each node is present; `graph` projects a
  collection, and projecting a graph yields it unchanged. The naming question in
  [graphs](../models/domain/graphs.md) stays open rather than being answered by
  the implementation.
- **The CLI requires a vault for card-ness** (`CON-08` for this host only). A
  document read outside one is a `Doc`; whether the language agrees is open.
- **`semantic` uses `hql.hashbag.v1`**: a hashed bag of words compared by
  cosine, offline, deterministic and exact, reporting `approximate: false`. It
  is step two of the path in [semantic search](../models/domain/semantic-search.md),
  not a stand-in for a trained model. Replacing the model changes the numbers,
  not the shape — which is the point of `Retrieval` being on every `Hit`.
- **A lambda is an argument, not a value.** `f = c => c` is an error. This keeps
  closures out of the value type entirely, and is the one deliberate narrowing
  of [pipes](../wiki/hql/types/pipes.hmd) made for the implementation's sake.

Not implemented, and said so rather than faked: `recall` over knowledge,
`hql#declare`, index pushdown, `match`, and any effect that writes.

See [the command line](../models/behavior/cli.md),
[reporting](reporting-modes.md), [orderable cards](orderable-cards.md) and
[absence and warnings](absence-and-warnings.md).
