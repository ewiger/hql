<!-- hql reference: search/semantic.semantic -->
# `semantic`

In [`search/semantic`](index.md).

```hql
fn semantic<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
```

The cards of `self` ranked against `query` by the cosine of their
vectors in the vault's index, as a `Ranking<Card>` in descending order
of score; a card scoring zero or less is not a hit. `query` must be in
the index. A card the index holds no vector for is left out of the
ranking, with a warning; one changed since the index was built is
scored, with a warning that its score is stale.
<!-- hql reference: end -->
