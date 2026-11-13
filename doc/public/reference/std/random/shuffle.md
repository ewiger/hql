<!-- hql reference: std/random.shuffle -->
# `shuffle`

In [`std/random`](index.md).

```hql
fn shuffle<T>(self: Seq<T>, *, seed: Int) : List<T>
effectful fn shuffle<T>(self: Seq<T>) : List<T>
```

The elements of `self` in a uniformly random order (Fisher–Yates).
<!-- hql reference: end -->
