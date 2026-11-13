<!-- hql reference: std/seq.one -->
# `one`

In [`std/seq`](index.md).

```hql
fn one(self) : T
```

The one element of `self`: a sequence of exactly one. Any other count is
an evaluation failure naming it, as `x | T` fails where `x` is not a
`T`; where absence is a case to handle, `first` gives an `Option<T>`.
`[7] | one` is `7`.
<!-- hql reference: end -->
