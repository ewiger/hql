<!-- hql reference: std/io/transaction.rollback -->
# `rollback`

In [`std/io/transaction`](index.md).

```hql
effectful fn rollback(self, reason: String) : Never
```

Ends the attempt by rolling it back, with `reason` in the scope's `err`,
its `kind` `"rolled_back"`. Nothing the attempt wrote to a participant
remains: `tx.rollback("insufficient balance") given sender.balance < 100`.
<!-- hql reference: end -->
