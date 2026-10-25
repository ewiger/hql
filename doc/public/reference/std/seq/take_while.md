<!-- hql reference: std/seq.take_while -->
# `take_while`

In [`std/seq`](index.md).

```hql
fn take_while<S : Seq<T>>(self: S, by: T -> Bool) : S
fn take_while(self: Iterable<T>, by: T -> Bool) : List<T>
```

The leading values of `self` that `by` accepts, up to the first it
rejects: `[1, 2, 5, 1] | take_while(x => x < 3)` is `[1, 2]`. A sequence
comes back at its own type, and any other iterable as a `List`.
<!-- hql reference: end -->
