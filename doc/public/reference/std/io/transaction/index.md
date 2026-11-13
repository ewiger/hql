<!-- hql reference: std/io/transaction -->
# `std/io/transaction`

```hql
module io.transaction for Transaction
```

The control calls of a transaction scope's handle, `tx`, bound by
`with transaction tx on participants { … }` (module `io`): each ends the
attempt and gives no value, so it stands where any type is expected and
nothing after it runs. A block that reaches its end commits with its
final value, so a scope needs none of them. In the prelude.

- [`commit`](commit.md) — Ends the attempt by committing it, with `value` as the scope's `ok`.
- [`rollback`](rollback.md) — Ends the attempt by rolling it back, with `reason` in the scope's `err`, its `kind` `"rolled_back"`.
- [`retry`](retry.md) — Ends the attempt by rolling it back and runs the block again on a fresh transaction, as the next attempt, while the scope's `attempts` remain; with none remaining the scope's `err` has the `kind` `"exhausted"`.
<!-- hql reference: end -->
