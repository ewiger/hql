<!-- hql reference: std/retrieval -->
# `std/retrieval`

```hql
module retrieval
```

Retrieval: a corpus ranked against a query. A score is evidence for the
proposition that a value is relevant to a query, never relevance itself,
so every hit carries the retrieval that produced it: the index, model
and metric, held as an open tree.

- [`Hit`](types/Hit.md) — One ranked value: the `value`, its `score` against `query`, and the `retrieval` that produced it, a tree holding `index`, `model`, `revision`, `metric`, `approximate` and `query`.
- [`Ranking`](types/Ranking.md) — The hits of one retrieval, as a `Seq<Hit<T>>` in descending order of score, ties ordered by name, with the `query` they are relative to and the `retrieval` that produced them.
<!-- hql reference: end -->
