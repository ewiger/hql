# A card is orderable, which is how `CON-05` resolves

Recorded 2026-09-18 from a user decision while implementing `take`.

- The question was never `Set` versus `List`. It was whether a prefix of the
  vault can be reproducible without vault order becoming observable. It can,
  because **a card carries an ordering key of its own**: its name.
- So `cards : Set[Card]` stands, as [collections](../wiki/hql/collections.hmd)
  says, *and* `cards | take(5)` is well defined and reproducible. Discovering
  files still establishes no ordering guarantee; the order comes from the
  values, never from how they were found.
- `Orderable` is an ordinary **supertype**, which is how the user phrased it:
  `Card <: Orderable`, and likewise `Int`, `Float`, `String`, `Doc` and `Hit`.
  Collections and `Data` are not. It is not a bound, a capability or a trait —
  the key is a property of the value, not something registered for it from
  outside. See [no traits](no-traits.md).
- `take` over elements that do not narrow `Orderable` is a type error that says
  to sort with an explicit key first.
- A `Hit` orders by score descending, then by its card's key, so equal scores
  do not make a prefix depend on load order.
- **`take` over a `Seq` keeps the order the sequence already has.** Re-imposing
  the canonical order there would silently discard the `sort` that produced it,
  which was a real bug during implementation.
- Consequently the `List<Card>` reading in [Card](../wiki/hql/types/card-type.hmd)
  is superseded rather than reconciled, and `CON-05` is decided. `INC-04`'s
  third site is the corpus, which is not in this working tree.

Implemented in `src/types.rs` (`is_orderable`), `src/values.rs` (`order_key`)
and `src/evaluation.rs`. Tests in `tests/vault.rs`.
