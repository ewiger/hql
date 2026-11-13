<!-- hql reference: std/io.Transaction -->
# `Transaction`

In [`std/io`](../index.md).

```hql
type Transaction
```

The handle of a transaction scope, bound by `with transaction tx on …`
(HQL-0051). It has no fields and no constructor. It is used in its own
block, through `tx.commit(value)`, `tx.rollback(reason)` and
`tx.retry()` (module `io.transaction`), and does not leave it: it is not
a parameter, a field, an element or a result, as a `Window` is not.
<!-- hql reference: end -->
