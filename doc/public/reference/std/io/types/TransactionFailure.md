<!-- hql reference: std/io.TransactionFailure -->
# `TransactionFailure`

In [`std/io`](../index.md).

```hql
type TransactionFailure : DataContract { kind : String reason : String attempts : Int endpoint? : String detail? : Data }
```

Why a transaction scope did not commit, the `err` of its `Result`:
`kind` is `"rolled_back"` for `tx.rollback`, `"conflict"` for a
concurrent transaction after the last attempt, `"constraint"` for a row
the system refused, and `"exhausted"` for `tx.retry` with no attempt
left; `reason` is the `rollback` argument or what the system said;
`attempts` counts the attempts run; `endpoint` names the participant a
statement was over, where one was; `detail` holds what the system said,
its state code among it.
<!-- hql reference: end -->
