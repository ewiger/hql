<!-- hql reference: std/core.DataContract -->
# `DataContract`

In [`std/core`](../index.md).

```hql
abstract type DataContract : Data
```

The supertype of an open contract: a structural type declared
`type D : DataContract = {…}` is a view over `Data` with at least its
declared fields, and is validated as a contract. It is open by being
`Data`; no `Data` is written beside it.
<!-- hql reference: end -->
