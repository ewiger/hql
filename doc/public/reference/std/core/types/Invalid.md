<!-- hql reference: std/core.Invalid -->
# `Invalid`

In [`std/core`](../index.md).

```hql
type Invalid : DataContract { contract : String value : Data violations : List<Violation> }
```

A value that did not read as a contract, and every reason why: its
`contract`, the `value`, and the `violations`.
<!-- hql reference: end -->
