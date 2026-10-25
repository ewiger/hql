<!-- hql reference: std/collections.sort -->
# `sort`

In [`std/collections`](index.md).

```hql
fn sort<F<_> : Seq<_>, T : Orderable>(self: F<T>) : F<T>
fn sort<T : Orderable>(self: Collection<T>) : List<T>
fn sort<F<_> : Seq<_>, T, K : Orderable>(self: F<T>, by: T -> K) : F<T>
fn sort<T, K : Orderable>(self: Collection<T>, by: T -> K) : List<T>
fn sort<F<_> : Seq<_>, T>(self: F<T>, by: (T, T) -> Ordering) : F<T>
fn sort<T>(self: Collection<T>, by: (T, T) -> Ordering) : List<T>
```

The elements in ascending order: by their own value order, by the key
`by` gives each, or by a comparison `by` of two elements. A sequence
keeps its constructor; any other collection becomes a `List`.
`[3, 1, 2] | sort` is `[1, 2, 3]`, and
`[3, 1, 2] | sort(by = (a, b) => compare(b, a))` is `[3, 2, 1]`.
<!-- hql reference: end -->
