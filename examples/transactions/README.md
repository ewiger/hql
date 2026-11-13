# Transactions: a ledger in one SQLite file

The cases follow [HQL-0051](../../doc/proposals/HQL-0051/README.md). Each
opens two tables of one SQLite file, `ledger.db`, with the constructor of
`io.db.sqlite`, writes their opening rows with `target`, and changes them
with `insert`, `update` and `delete`. `with transaction tx on {accounts,
transfers} { … }` runs its block as one transaction over the two tables and
commits when the block ends; its value is a `Result<T, TransactionFailure>`,
and each case binds it or makes it the program's result, so no failure goes
unobserved.

The accounts open as ada with 150 and grace with 40, and one earlier
transfer is on record. A case that writes is run in a temporary copy of
this folder, so `ledger.db` is created there and the checkout is left as it
was; every case runs offline.

| Case | What it shows | Result |
| --- | --- | --- |
| [transfer](queries/transfer-commits.hql) | two updates and an insert committed together, each update's count read | balances `[50, 140]`, one transfer recorded |
| [insufficient balance](queries/insufficient-balance.hql) | `tx.rollback` on the sender's balance, matched as data | `rolled_back`, nothing written |
| [credit to nobody](queries/credit-to-nobody.hql) | an `update` matching no row counts 0, and the count check rolls the debit back | `rolled_back`, balances unchanged |
| [retry exhausted](queries/retry-exhausted.hql) | `tx.retry()` under `attempts = 3`, each attempt rolled back | `exhausted after 3 attempts` |
| [reads its own writes](queries/reads-own-writes.hql) | a `source` in the block sees the block's `update` | `ok([150, 45])` |
| [not a participant](queries/invalid-not-a-participant.hql) | `delete` of a table the set does not name | a check error |
| [nested scope](queries/invalid-nested-scope.hql) | a scope inside a scope | a check error |
| [two files](queries/invalid-two-files.hql) | participants in two database files | a check error |
| [handle in a lambda](queries/invalid-handle-in-a-lambda.hql) | `tx.rollback` inside `map` | a check error |

From the repository root, in a scratch folder so the database is written
there:

```sh
cargo build --locked
mkdir -p /tmp/ledger && cp examples/transactions/queries/transfer-commits.hql /tmp/ledger/
target/debug/hql run /tmp/ledger/transfer-commits.hql
```
