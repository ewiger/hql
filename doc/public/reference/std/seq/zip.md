<!-- hql reference: std/seq.zip -->
# `zip`

In [`std/seq`](index.md).

```hql
fn zip<U>(self, other: Seq<U>) : List<Pair<T, U>>
```

Pairs of the elements of `self` and `other` at equal positions, as long
as the shorter: `[1, 2, 3] | zip(["a", "b"])` is
`[Pair(first = 1, second = "a"), Pair(first = 2, second = "b")]`.

```hql
fn zip<U>(self: Stream<T>, other: Seq<U>) : Seq<Pair<T, U>>
fn zip<U>(self, other: Stream<U>) : Seq<Pair<T, U>>
fn zip<U>(self: Stream<T>, other: Stream<U>) : Stream<Pair<T, U>>
```

Pairs of the elements of `self` and `other` at equal positions, where
either is a `Stream`, until either ends: a `Stream` beside a `Seq` ends
with it, as a `Seq`, and two `Stream`s give a `Stream`.
<!-- hql reference: end -->
