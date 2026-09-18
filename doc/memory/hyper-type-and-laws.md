# `HyperType`, and where the laws are enforced

The type system card's *Where the laws live* table puts enforcement in `src/`,
and conceded that the third column carried the whole weight while the first two
were "prose that a test cannot contradict". `src/types/` is now that column.

`src/types.rs` and `src/values.rs` became one module: `lattice.rs` (the `Type`
enum, unchanged), `value.rs` (the `Value` enum), `display.rs` (printing, JSON,
tables — moved out of `values.rs`, which had been carrying them), `hyper.rs`
(the markers), `laws.rs`.

Two traits, both host mechanisms that do not reach HQL's surface:

- `HyperType` — a marker per type HQL has, giving the lattice type it stands
  for. The method is `lattice()`, after the existing function of that name in
  `declarations.rs`; `reify()` was considered and dropped as a second word for
  a job already named.
- `Carried: HyperType` — the Rust value a type's values are made of, plus the
  `read`/`wrap` round trip.

**`Orderable` implements the first and not the second.** The card observes in
prose that it is the one `Type` case with no `Value` variant, being an abstract
type of law and nothing else. Splitting the trait makes that a compile-time
fact: `Orderable::read` does not exist.

Law checking has two arms, as the card's distinction between a law implemented
*as defined* and one honored *behaviorally* requires:

- compile time — `Narrows<Super>` is a witness the compiler discharges.
  Covariance is a generic impl, so `Seq<ConceptCard> <: Seq<Doc>` follows from
  `ConceptCard <: Doc` rather than being restated.
- run time — `laws::{narrowing, round_trips, inhabits, orderable}` quantify
  over values, which no Rust bound reaches. `narrowing` holds the compile-time
  witnesses against `Type::is` so the two accounts cannot drift.

Two findings the laws surfaced, both real and both predating the trait:

- **Presence is unmarked.** There is `Value::Absent` and no counterpart for
  presence, so `Option<Int>` holding a value *is* a `Value::Int` and reports
  its type as `Int`. `Option<T>` therefore fails `inhabits` and `orderable`.
- **`Hit` and `Ranking` erase their element.** Both are parametric in the
  lattice and monomorphic in the carrier — `Value::type_of` answers
  `Hit<Card>` for every hit — so only `Hit<Card>` can be `Carried`.

Removing `Type::Ranking` is still wanted. The cost to weigh first: it is what
gives `ranking.hits`, `ranking.query` and `ranking.retrieval` a type in
`checker::field_type`, and a ranking reified as `Seq<Hit<Card>>` loses them.
Each hit carries its own provenance, so the data survives; the top-level field
access does not.
