<!-- hql reference: std/random.sample -->
# `sample`

In [`std/random`](index.md).

```hql
fn sample<T>(self: Seq<T>, n: Int, *, seed: Int) : List<T>
fn sample<T : Orderable>(self: Collection<T>, n: Int, *, seed: Int) : List<T>
effectful fn sample<T>(self: Seq<T>, n: Int) : List<T>
effectful fn sample<T : Orderable>(self: Collection<T>, n: Int) : List<T>
```

`n` elements of `self` drawn without replacement, in their order in
`self`; all of `self` where `n` is not below its size. A negative `n` is
an evaluation error. A `Collection` without an order is sorted by its
elements first, so the sample does not depend on how it was built.

```hql
effectful fn sample<T>(self: Stream<T>, n: Int) : List<T>
```

`n` elements of a stream, drawn by reservoir sampling: the step holds
`n` elements, never the stream, and gives its sample when the stream
ends. A stream that does not end gives no sample.
<!-- hql reference: end -->
