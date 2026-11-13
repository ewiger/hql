<!-- hql reference: std/core.Option -->
# `Option`

In [`std/core`](../index.md).

```hql
type Option<T>
```

An optional value of `T`: present, holding one `T`, or absent. Absence is
an ordinary sum type; there is no null. The ways out are the `option`
module's, and each says what absence means.
<!-- hql reference: end -->
