# A transfer is one transaction

The shop pays its suppliers from a ledger: an `accounts` table and a
`transfers` table in the one PostgreSQL database of
[chapter 16](../16-tables-and-topics.md). A transfer of 100 from one account
to another changes two rows and adds one, and the three changes hold
together or not at all. `target` cannot say that: it rewrites a table whole,
and three writes are three commits. This page is the statements that change
rows in place, and the scope that makes several of them one transaction
([HQL-0051](https://github.com/ewiger/hql/blob/main/doc/proposals/HQL-0051/README.md)).

Every program here is checked, not run: the book's tests connect to
nothing. The two records are declared once:

<!-- define: Account -->
```hql
type Account { id : Int, owner : String, balance : Decimal }
```

<!-- define: Transfer -->
```hql
type Transfer { id : Int, from_id : Int, to_id : Int, amount : Decimal }
```

The configuration names each table's record, `element = "Account"`, beside
its connection:

```toml
[endpoint.accounts]
kind = "postgres"
dsn_env = "SHOP_DSN"
table = "accounts"
order = ["id"]
element = "Account"

[endpoint.transfers]
kind = "postgres"
dsn_env = "SHOP_DSN"
table = "transfers"
order = ["id"]
element = "Transfer"
```

## Changing rows in place

`configured("accounts")` is the table itself as a value, where
`source("accounts")` is its rows. Three statements change what a table
holds. `update` selects rows with the `filter` of every pipeline and `set`
gives the fields that change; its value is the number of rows changed:

<!-- run: check with Account,Transfer --vault examples/pipelines/shop -->
```hql
accounts = configured("accounts")

update accounts
| filter(a => a.id == 1)
| set(a => {balance: a.balance - 100.00})
```

```text
Int
```

The lambdas are not run row by row. The statement runs on the database as
one `UPDATE … SET … WHERE …`, so a lambda holds only what translates: a
field of the row, a literal, a value bound outside the lambda, comparisons,
`and`, `or`, `not`, arithmetic, `contains` of a bound collection, and
`is_none` or `is_some` of a field. Anything else is a check error that says
to compute it in HQL and bind it. `insert` adds a row or a collection of
them, and `delete` removes the rows it selects:

<!-- run: check with Account,Transfer --vault examples/pipelines/shop -->
```hql
transfers = configured("transfers")

added = insert transfers Transfer(id = 1, from_id = 1, to_id = 2, amount = 100.00)
gone  = delete transfers | filter(t => t.amount == 0.00)

[added, gone]
```

```text
List<Int>
```

A filter that matches no row is no failure: the statement changes nothing
and its value is `0`. A program that needs the row to exist reads the
count. A statement outside a transaction is its own transaction, committed
when it ends, as `target` of a table is. The checker refuses a statement
over an endpoint that cannot take it: a Kafka topic is not `Updatable`, a
CSV file is not `Deletable`. A `sqlite` or `postgres` table is all three, and a
`spark` table too, where the table's format allows it.

## Two updates and an insert, or none

A transaction scope runs its block as one unit over its participants, a set
of endpoints in one database, and commits when the block ends:

<!-- run: check with Account,Transfer --vault examples/pipelines/shop -->
```hql
accounts = configured("accounts")
transfers = configured("transfers")
ledger = {accounts, transfers}

with transaction(isolation = Serializable) tx on ledger {
    sender = source(accounts) | filter(a => a.id == 1) | one
    tx.rollback("insufficient balance") given sender.balance < 100.00

    debited = update accounts | filter(a => a.id == 1) | set(a => {balance: a.balance - 100.00})
    credited = update accounts | filter(a => a.id == 2) | set(a => {balance: a.balance + 100.00})
    tx.rollback("no such account") given debited != 1 or credited != 1

    insert transfers Transfer(id = 1, from_id = 1, to_id = 2, amount = 100.00)
}
```

```text
Result<Int, TransactionFailure>
```

Inside the block every read and write of a participant runs on the scope's
own connection: a `source` after an `update` sees the update, and a `sink`
commits no row of its own. `one` gives the one element of a sequence of
exactly one, and fails otherwise. Each `update` is bound so that its count
is read: a credit to an account that does not exist changes no row, and
without the check the scope would commit the debit alone.

`isolation = Serializable` is what keeps the balance check true when two
transfers run at once. Under it, two scopes that both read a balance of
150 and both debit 100 cannot both commit: the database refuses one of
them as a conflict. PostgreSQL's default level, `ReadCommitted`, does not
refuse it: the second debit waits for the first, then subtracts from the
committed 50, and the balance ends at -50. SQLite has no other level.

The handle `tx` ends the attempt early:
`tx.rollback(reason)` undoes it and gives the reason back,
`tx.commit(value)` commits before the end, and `tx.retry()` runs the block
again. Each is written as a statement of the block or as an arm of a
`given … otherwise` or a `match` that is one, and `tx` goes nowhere else:
not into a lambda, not into a function.

The scope's value is a `Result`: `ok` with the block's final value, here the
rows the `insert` added, and `err` with a `TransactionFailure` saying why
the scope did not commit. A rollback by the program's own rule, a
constraint the database enforced and a conflict that outlasted the attempts
are all data, matched like any other:

<!-- run: check with Account,Transfer --vault examples/pipelines/shop -->
```hql
accounts = configured("accounts")
transfers = configured("transfers")

outcome =
    with transaction(attempts = 3, isolation = Serializable) tx on {accounts, transfers} {
        sender = source(accounts) | filter(a => a.id == 1) | one
        tx.rollback("insufficient balance") given sender.balance < 100.00
        debited = update accounts | filter(a => a.id == 1) | set(a => {balance: a.balance - 100.00})
        credited = update accounts | filter(a => a.id == 2) | set(a => {balance: a.balance + 100.00})
        tx.rollback("no such account") given debited != 1 or credited != 1
        insert transfers Transfer(id = 1, from_id = 1, to_id = 2, amount = 100.00)
    }

match outcome {
    ok(n) => "transferred",
    err(f) => f.kind + ": " + f.reason,
}
```

```text
String
```

`attempts = 3` runs the block again, on a fresh transaction, when the
database refuses an attempt because of a concurrent one: in PostgreSQL a
serialization failure under `isolation = Serializable` or a deadlock, in
SQLite a database file another transaction is writing. The attempt is
retried with no delay, whether a statement or the commit was refused, and
after the last one the scope is `err` with `kind = "conflict"`. A retried
attempt reads the balance again, so a transfer that lost the race to
another one meets the balance that one left, and rolls back by its own
rule where it no longer holds. HQL's own tests run that race over SQLite:
two of these transfers of 100 start together from a balance of 150, both
pass the balance check, one commits, and the other rolls back on the
balance after a retry or ends `conflict`, with no half of it left. A
constraint violation is never retried, since the same rows would be
refused again.
Any other failure inside the block, a division by zero, a lost connection,
rolls the attempt back before it is reported, so a program that stops
leaves no half-made transfer.

The [`ledger` case](https://github.com/ewiger/hql-demo/tree/main/ledger)
of hql-demo runs this transfer against PostgreSQL 17: twice it commits,
and the third time it rolls back on the sender's balance.

## What the kind promises

The statements read the same over every table; what a scope guarantees is
the kind's.

| Kind | One scope holds | `insert`, `update`, `delete` |
| --- | --- | --- |
| `postgres` | every table of one database, at `ReadCommitted` unless the scope names `RepeatableRead` or `Serializable` | all three, as one statement each |
| `sqlite` | every table of one database file, at `Serializable`, the one level it offers | all three; a `Decimal` computed on the server is a float |
| `spark`, a Delta table | one table, and at most one statement that changes it: a Delta table commits one statement of itself | all three where the table's format allows it |

A set of two kinds, or two databases, is refused where the program is
checked: no scope spans them, and nothing runs weaker than it was asked
to. Two Delta tables that must change together are two commits, and the
scope has no form for it; coordinating them is a later record.

## What is next

[Chapter 20](../20-olap/index.md) turns from the transactions to what they
leave behind, and its [merge page](../20-olap/merge.md) is the analytical
counterpart of `update`: a table kept up to date by key, whole.
