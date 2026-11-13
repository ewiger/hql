<!-- hql reference: std/collections.take -->
# `take`

In [`std/collections`](index.md).

```hql
fn take<S : Seq<T>, T>(self: S, n: Int) : S
fn take<T>(self: Stream<T>, n: Int) : List<T>
fn take<T : Orderable>(self: Collection<T>, n: Int) : List<T>
```

The first `n` elements. A prefix of a sequence keeps its positions and
its type: `[5, 3, 4] | take(2)` is `[5, 3]`. A prefix of a stream is a
finite `List`; a prefix of any other collection is taken in its
elements' order: `Set([5, 3, 4]) | take(2)` is `[3, 4]`. A negative `n`
is an evaluation error.
<!-- hql reference: end -->
