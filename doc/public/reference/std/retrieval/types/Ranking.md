<!-- hql reference: std/retrieval.Ranking -->
# `Ranking`

In [`std/retrieval`](../index.md).

```hql
type Ranking<T> : Seq<Hit<T>> { hits : List<Hit<T>> query : String retrieval : Data }
```

The hits of one retrieval, as a `Seq<Hit<T>>` in descending order of
score, ties ordered by name, with the `query` they are relative to and
the `retrieval` that produced them. A value that scores zero is not a
hit.
<!-- hql reference: end -->
