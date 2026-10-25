<!-- hql reference: std/seq.partition -->
# `partition`

In [`std/seq`](index.md).

```hql
partition fn partition<S : Seq<T>, K>(self: S, by: T -> K) : S
partition fn partition<K>(self: Stream<T>, by: T -> K) : Stream<T>
```

The values of `self` redistributed by the key `by` gives: each value
once, at its own type, those of one key in one logical partition. The
step is a stage boundary. The local backend has one partition per key on
one worker and passes the values on in their order.
<!-- hql reference: end -->
