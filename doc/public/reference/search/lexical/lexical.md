<!-- hql reference: search/lexical.lexical -->
# `lexical`

In [`search/lexical`](index.md).

```hql
fn lexical<T : Doc>(self: Collection<T>, query: String) : Ranking<Card>
```

The cards of `self` ranked by the words their text shares with
`query`, as a `Ranking<Card>`. The text of each, and `query`, is a
hashed bag of its words of two or more letters or digits, lowercased,
and the two are compared by cosine. A card that shares no word with
`query` scores zero and is not a hit; the rest are in descending order
of score. The retrieval names the model `hql.hashbag.v1` and the metric
`cosine`, and is not approximate.
<!-- hql reference: end -->
