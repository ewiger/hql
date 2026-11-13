# HQL-0052: Heterogeneous transaction coordination

**Status**: drafted
**Created**: 2026-11-13
**Source**: [HQL-0051](../HQL-0051/README.md) (the transaction scope, its
handle, its lifecycle and the kinds' scope rules), [HQL-0021](../HQL-0021/README.md)
(endpoints, effects and what a host grants), [HQL-0047](../HQL-0047/README.md)
and [HQL-0048](../HQL-0048/README.md) (transports, their modules, `io.store`),
[HQL-0046](../HQL-0046/README.md) and [HQL-0049](../HQL-0049/README.md)
(topics and tables as configured endpoints), [HQL-0033](../HQL-0033/README.md)
(errors as values, `retry`), [HQL-0045](../HQL-0045/README.md) (failures as
data or diagnostics), [HQL-0050](../HQL-0050/README.md) (a value that does not
leave its scope), [HQL-0024 part 5](../HQL-0024/parts/05-traits.md) and
[HQL-0039](../HQL-0039/README.md) (traits are not types),
[HQL-0029](../HQL-0029/README.md) (sets), `lib/std/io/mod.hql`,
`src/endpoints.rs`, `src/postgres.rs`

## Abstract

A transaction scope of [HQL-0051](../HQL-0051/README.md) gains one named
argument, `strategy`, passed as a type, as `isolation` is. It takes one of three
values: `Atomic`, `Staged` or `Saga`. `Atomic` is the default and is
HQL-0051 unchanged, extended in one direction only: participants in
different systems form one atomic scope when every one of them can prepare a
commit and the scope names a durable journal, by two-phase commit. `Staged`
writes every participant's output unpublished and then publishes it through
one coordinated mechanism. A reader that follows the mechanism sees all of it
or none. `Saga` commits each effect on its own and records it in a journal.
On failure it runs each committed effect's compensation in reverse order.
It promises recovery, not atomicity. Five abstract types join `io` as
capabilities of a transport: `PreparedCommit`, `StagedWrite`,
`IdempotentWrite`, `Compensatable` and `Publishing`. A participant set of
mixed kinds is typed member by member, so the checker knows each member's
kind. At check time, a **coordination planner** compares the requested strategy with the
members and the block's statements. It returns a plan, or a check error
that names the member that breaks the guarantee. It never returns a weaker
strategy than the one written. `TransactionFailure` gains four `kind`s and
an `effects` field. Nothing here is implemented; every construct is proposed.

## Motivation

- One business fact often lands in four places. A completed order is a row
  in PostgreSQL, a row in a Delta table for analytics, an event on a Kafka
  topic and a JSON document in an archive. Written today as an `update`, an
  `insert`, a `sink` and a `target`, they are four commits with no shared
  outcome. If the third one fails, the first two stand and nothing records
  that the fourth was owed.
- [HQL-0051](../HQL-0051/README.md) answers "which statements form one
  transaction" for one system, and refuses the rest at check time. Its
  section "OLTP and OLAP" says so plainly: "Two tables are two commits, and a
  program that needs both or neither has no form for it here." This record
  is that form, and it keeps HQL-0051's rule that nothing runs weaker than it
  was asked to.
- The systems already have the pieces, and none of them spans two systems on
  its own:
  - PostgreSQL prepares a transaction for an external manager with
    [`PREPARE TRANSACTION`](https://www.postgresql.org/docs/current/sql-prepare-transaction.html),
    which "is not intended for use in applications or interactive sessions"
    but for "an external transaction manager to perform atomic global
    transactions across multiple databases or other transactional
    resources". It is disabled unless `max_prepared_transactions` is above
    zero, and a prepared transaction left unresolved holds its locks and
    interferes with `VACUUM`.
  - Kafka's transactional producer writes to several partitions and commits
    consumer offsets atomically, and a `read_committed` consumer sees all of
    it or none ([Kafka, "Message Delivery Semantics"](https://kafka.apache.org/documentation/#semantics)).
    It joins no other system's transaction.
  - Databricks commits several Unity Catalog managed tables in one
    transaction, `BEGIN ATOMIC … END` or an interactive `BEGIN TRANSACTION`,
    where every table has catalog commits enabled. The feature has been in
    public preview since March 2026
    ([Databricks, "Transactions"](https://docs.databricks.com/aws/en/transactions/),
    [catalog commits](https://docs.databricks.com/aws/en/tables/features/catalog-commits)).
    The OLAP side now has a multi-table atomic scope, inside one catalog.
  - Amazon S3 updates one key atomically, and "there is no way to make
    atomic updates across keys"
    ([Amazon S3, "data consistency model"](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)).
- A coordinator written by hand around these pieces, in a job scheduler or in
  application code, does not know the kinds of the endpoints it coordinates. HQL does: every
  capability is a supertype the checker reads
  ([HQL-0021](../HQL-0021/README.md)). That makes it possible to check,
  before anything runs, which guarantee a set of endpoints can jointly give.

## Goals

- One syntax for every strategy: the `with transaction` scope of HQL-0051,
  its handle and its three control calls, with the strategy written in the
  head beside `attempts` and `isolation`.
- The checker states, before the first effect, which guarantee a scope gives:
  it names the strategy, the mechanism for each member, and what a reader
  outside the protocol sees.
- No silent fallback. A scope that asks for `Atomic` and cannot have it is a
  check error, never a saga.
- A partial failure is recorded durably, so a later run can continue from it,
  and is reported as data.

## Non-goals

- Universal ACID. A Kafka event once read cannot be unread, and an e-mail
  once sent cannot be unsent; this record names such effects, it does not
  pretend to undo them.
- XA or two-phase commit with a broker. No broker of HQL-0048 prepares a
  commit for an external manager.
- A general workflow engine: timers, signals, human steps, long sleeps.
  Evaluation reads no clock ([HQL-0033](../HQL-0033/README.md)). The section
  "Progression" places durable execution as a later record.
- Exactly-once delivery to an endpoint that offers no idempotency.
  `IdempotentWrite` is declared by a kind, and is not inferred.
- Nested meta-transactions and savepoints, which HQL-0051 already reserves.
- Changing what a scope with no `strategy` means. It is HQL-0051's scope.

## Specification

### 1. What exists, what is specified, what is proposed

Each construct this record uses is one of three things.

| Construct | Standing |
| --- | --- |
| `Endpoint`, `Readable`, `Writable`, `Transport`, `File`, `file(path)` | implemented, `lib/std/io/mod.hql` |
| `Kafka`, `kafka(topic, …)` | specified by [HQL-0047](../HQL-0047/README.md); kind implemented, [HQL-0046](../HQL-0046/README.md) |
| `Postgres`, `postgres(dsn_env, table, …)`, `Spark` | specified by [HQL-0047](../HQL-0047/README.md) and [HQL-0048](../HQL-0048/README.md) |
| `with transaction(…) tx on … { … }`, `Transaction`, `commit`, `rollback`, `retry`, `TransactionFailure`, `Transactional`, `Insertable`, `Updatable`, `Deletable`, `insert`, `update`, `delete`, `configured("id")` | drafted, [HQL-0051](../HQL-0051/README.md) |
| a Delta or Iceberg table as a kind over `io.store`; `s3` | design direction, [HQL-0048](../HQL-0048/README.md) |
| `strategy`, `CoordinationStrategy`, `Atomic`, `Staged`, `Saga` | **proposed** here |
| `PreparedCommit`, `StagedWrite`, `IdempotentWrite`, `Compensatable`, `Publishing` | **proposed** here |
| the participant set typed member by member | **proposed** here |
| the coordination planner and `hql check --explain` | **proposed** here |
| `journal` and `id` scope arguments, the journal's record | **proposed** here |
| `compensate { … }` and `compensate none` clauses | **proposed** here |
| the `TransactionFailure` kinds `in_doubt`, `compensated`, `compensation_failed`, `publication_failed`, and `effects` | **proposed** here |

Every example below that uses a construct proposed here or in HQL-0051 is
marked `// proposed` on its first line. A constructor that is design
direction, `delta(…)` or `s3(…)`, is marked `// design direction` as well.

### 2. Terms

Each term has one meaning, and each extends a term of
[HQL-0051](../HQL-0051/README.md), "Terms".

- **commit domain**: a set of endpoints one system commits as a whole: the
  tables of one PostgreSQL database, of one SQLite file, of one catalog with
  catalog commits. HQL-0051 calls it an atomic scope.
- **atomic transaction**: a scope whose participants share one commit
  domain. It is HQL-0051's scope, and its guarantee is the system's.
- **coordinated meta-transaction**: a scope whose participants span more
  than one commit domain. Its guarantee is the coordinator's and is the
  strategy's, never more.
- **coordinator**: the part of the HQL runtime that runs a meta-transaction:
  it begins each domain's local transaction, orders the effects, records
  them in the journal, and decides the outcome.
- **journal**: a `Transactional` endpoint where the coordinator durably
  records a meta-transaction's progress, so a later run can recover it.
- **effect**: one effectful statement of the block over a participant: a
  mutation, a `target` or a `sink`. Each effect has an **effect identity**,
  derived from the scope's `id`, the statement's position and the attempt.
- **publication**: the act that makes staged output visible to readers that
  follow the publication mechanism.
- **compensation**: an effect that semantically undoes an earlier,
  committed effect: a delete for an insert, a correcting event for an event.
  It is a new effect, not an erasure.
- **pivot**: an effect with no compensation. After it commits, the
  meta-transaction can only move forward.

### 3. The strategy

The strategy is a named argument of the scope, passed as a type, as
`isolation` is ([HQL-0051](../HQL-0051/README.md), "The scope's arguments").

```hql
// proposed: lib/std/io/mod.hql
/// What a transaction scope promises when it commits. The default is
/// `Atomic`; the others are written to ask for less, and say so.
abstract type CoordinationStrategy
type Atomic : CoordinationStrategy
type Staged : CoordinationStrategy
type Saga   : CoordinationStrategy
```

```hql
// proposed
with transaction(strategy = Saga, id = order_key, journal = runs) tx on destinations { … }
```

| Strategy | `tx.commit()` means | On failure before the decision | On failure after the decision |
| --- | --- | --- | --- |
| `Atomic` | every participant commits, or none does | every participant rolls back | recovery commits every participant, from the journal |
| `Staged` | every participant's output is published by one act | staged output is discarded; nothing was visible | recovery finishes the publication; a reader following the mechanism never sees part of it |
| `Saga` | every effect has committed, in order | committed effects are compensated in reverse order | after a pivot, recovery retries the remaining effects forward |

- `strategy` MUST be one of the three types. Without it the scope is
  `Atomic`, and an `Atomic` scope over one commit domain MUST be exactly
  HQL-0051's scope: no journal, no `id`, no coordinator.
- A written strategy is a **requirement**, not a hint. The planner MUST
  either realize it or refuse the scope. It MUST NOT choose a weaker
  strategy. The ordering is `Atomic` above `Staged` above `Saga`.
- The planner MAY realize a weaker request with a stronger mechanism where
  one exists: a `Saga` over one commit domain runs as HQL-0051's scope, and
  `hql check --explain` says so. Asking for less never gives less than was
  asked.
- `using Saga` after the participant set was considered and not taken: it
  adds a positional word, and it splits the scope's configuration between
  the head and the tail. A scope is configured in one place.

### 4. Capabilities

The five capabilities of this record are abstract supertypes, not traits.
Every reason [HQL-0051](../HQL-0051/README.md), "Capabilities are abstract
types, not traits", gives applies unchanged: every endpoint capability is a
supertype today, a trait is not a type, and no generic HQL code quantifies
over them.

```hql
// proposed: lib/std/io/mod.hql, beside Transactional

/// A `Transactional` transport whose system prepares a local transaction
/// for an external coordinator: after `prepare` the commit survives a crash
/// and is decided later, from any session. PostgreSQL's
/// `PREPARE TRANSACTION`.
abstract type PreparedCommit : Transport

/// A transport that writes output nobody sees yet and later makes it
/// visible, or discards it, by one act of its own: an uncommitted lakehouse
/// snapshot, an open producer transaction, an object under a temporary key.
abstract type StagedWrite : Transport

/// A transport where repeating a write that carries the same effect
/// identity leaves the endpoint as one write would have: a keyed upsert,
/// an idempotent producer, an object put under a deterministic key.
abstract type IdempotentWrite : Transport

/// A transport that derives the compensation of its own mutations: the
/// delete for an `insert`, the guarded restore for an `update`, the
/// re-insert for a `delete`.
abstract type Compensatable : Transport

/// A transport whose readers follow a publication mechanism the
/// coordinator can drive: a manifest, a catalog pointer, an outbox.
abstract type Publishing : Transport
```

- The five MUST be independent of one another and of HQL-0051's four. A kind
  declares any subset. `PreparedCommit` without `Transactional` is a check
  error on the declaration, since there is no local transaction to prepare.
- `Compensatable` says that a kind **derives** compensations. Whether an
  effect can be compensated depends on the operation, not on the endpoint,
  and the section "Compensation is per effect" decides it statement by
  statement. A topic is not `Compensatable`, yet an event on it can have a
  compensation that the program writes.
- `IdempotentWrite` is a claim of the kind, made only where the system
  enforces it. The coordinator relies on it in order to retry an effect
  after a crash without duplicating it.

The kinds gain supertypes and lose none.

| Kind | Commit domain ([HQL-0051](../HQL-0051/README.md)) | `PreparedCommit` | `StagedWrite` | `IdempotentWrite` | `Compensatable` | `Publishing` | Standing |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `postgres` | every table of one `dsn_env` | yes, where `max_prepared_transactions > 0` | yes: a prepared transaction is staged output | yes, for `insert` into a table with a key, as `INSERT … ON CONFLICT DO NOTHING` on the effect identity | yes | yes, as an outbox table | this record |
| `sqlite` | every table of one `path` | no | no | yes, as `postgres` | yes | no | this record |
| `spark` over a Delta table | one table | no | no | no | yes, for `insert` by key | no | this record |
| Delta, Iceberg over a catalog with catalog commits | every catalog-committed table of one catalog | no | yes: the catalog transaction before its commit | yes, by `MERGE` on the effect identity | yes | yes: the catalog commit | design direction |
| Delta, Iceberg over `io.store`, no catalog | one table: a snapshot commit | no | yes: data files written, the snapshot not yet committed | yes, as above | yes | yes: the snapshot pointer | design direction |
| `kafka` | none | no | yes, for `read_committed` readers: an open producer transaction | yes: the idempotent producer, within one producer session | no | no | this record |
| `nats_jetstream` | none | no | no | yes, by the `Nats-Msg-Id` header within the stream's duplicate window | no | no | this record |
| `nats` | none | no | no | no | no | no | — |
| `file`, `s3` and other object stores | none | no | yes: an object under a temporary key | yes: a put under a key derived from the effect identity | yes: the previous content restored, or the object deleted | yes, through a manifest object | `file` this record; `s3` design direction |
| `http`, `rest`, `webhook` | none | no | no | no, unless a kind declares an idempotency-key header | no | no | — |

The `kafka` row deserves its caveat in the table itself. A Kafka producer
transaction left open by a crashed process is aborted when a new producer
with the same transactional id starts. Staged output on a topic therefore does not
survive the coordinator, and the coordinator MUST keep the payload in the
journal until the publication is recorded (the section "Staged").

### 5. A participant set of mixed kinds

The planner decides by kind, so it needs each member's kind. HQL-0051,
"The participants", observes that `{accounts, ledger_file}` with a
`Postgres` and a `Sqlite` has no type today: the common type of two
declared types is one of them or `Data`, never a shared ancestor. This
record gives such a set a type in exactly one place.

- In the `on` position, a brace literal of endpoint values, or a binding
  of one, MUST be typed **member by member**: the checker keeps each
  member's own type and name. The type is written
  `Participants{orders: Postgres, analytics: Delta, events: Kafka, archive: File}`
  in a diagnostic and in `hql check --explain`.
- A binding whose value has a `Participants` type MAY be used only in the
  `on` position of a scope, any number of times. It is not iterable, not an
  element, a field, a parameter or a result, by the rule
  [HQL-0050](../HQL-0050/README.md) gives `Window<T>`. That rule is what
  keeps the members' kinds from being erased.
- A value of type `Set<Endpoint>` or `Set<Transactional>`, written with an
  annotation or received from elsewhere, MUST be refused in the `on` position,
  naming the erasure: `` the scope needs each participant's kind, and
  `Set<Endpoint>` has erased them; write the set where the scope can see it ``.
- The set MUST still be known at check time (HQL-0051), and a member MUST
  be a `Transport`. Under `Atomic` with one commit domain, HQL-0051's rule
  applies unchanged; the member-by-member type only reaches the planner.

This answers HQL-0051's open question about a brace literal with an
annotation for scopes alone. Any other set of mixed kinds is unchanged.

### 6. The scope's new arguments

Two named arguments join `attempts`, `isolation` and `strategy` in the head.

```hql
// proposed
runs = postgres(dsn_env = "OPS_DSN", table = "hql_journal")

with transaction(strategy = Saga, id = fmt("order-{}", order_id), journal = runs) tx
on destinations { … }
```

- `journal : E` with `E : Transactional`. It is where the coordinator records
  the meta-transaction. It MUST be given wherever the plan has more than one
  commit domain, and MUST NOT be given where it has one, since HQL-0051's
  scope needs none. The journal MAY be in the same commit domain as a
  participant, and the planner uses that (the section "The planner").
- `id : String`. It is the meta-transaction's identity, stable across runs
  of the program. It MUST be given wherever `journal` is given. Two runs
  with one `id` are one meta-transaction: the second **recovers** the
  first, it does not repeat it. A scope over data of many orders binds `id`
  per order.
- An `id` MUST be computable before the block runs, from bindings outside
  it. An `id` that depends on a read inside the block would change between
  attempts.

### 7. The planner

The planner runs in the checker. It takes the scope's strategy, the members
and their kinds, the kinds' commit-domain rules, and the block's effects in
program order. It produces a **plan**: a commit domain for each member and a
mechanism for each domain. Otherwise it produces a check error.

```text
plan(strategy, members, effects):
  domains := group members by the kind's commit-domain rule
  if |domains| = 1:
      return native(domains[0])                          -- HQL-0051, any strategy
  match strategy:
    Atomic:
      every domain is PreparedCommit, and journal given  -> two_phase(domains)
      otherwise -> error naming the first domain that is not PreparedCommit
    Staged:
      every domain is StagedWrite, and one mechanism publishes all of them
                                                        -> staged(domains, mechanism)
      otherwise -> error naming the domain with no publication path
    Saga:
      effects split as  C* P? R*   (see "Compensation is per effect")
      and every effect in R is IdempotentWrite or carries the effect identity,
      and journal given                                 -> saga(effects)
      otherwise -> error naming the first effect out of place
```

- The planner MUST be deterministic: the same program and configuration
  give the same plan.
- A refusal MUST name the strategy, the member or effect that breaks it, and
  the weakest strategy the set could have. That last item lets the
  program see the choice, while the planner does not make it.

  ```text
  error[invalid-argument]: no `Atomic` scope holds these participants
    --> order.hql:14:5
     |
  14 | with transaction tx on destinations {
     |                        ^^^^^^^^^^^^ `events` is a `Kafka`, which cannot prepare a commit
     |
     = these participants span 4 commit domains: orders (postgres SHOP_DSN),
       analytics (delta catalog main), events (kafka), archive (file)
     = they can be coordinated as `strategy = Saga`; write it if that is the guarantee you want
  ```

- `hql check --explain` MUST print the plan of every scope: the domains, the
  mechanism of each, the journal, the effect order with each effect's
  compensation or `pivot`, and one sentence on what a reader outside the
  protocol observes.
- Where the journal is in one commit domain with participants, the plan
  MUST write the journal's records in that domain's local transaction. A
  saga step and its record then commit together, which removes the gap
  between "done" and "recorded" for that domain.

### 8. Atomic across commit domains: two-phase commit

`Atomic` over several domains is two-phase commit, with the HQL runtime as
the transaction manager and the journal as its log.

```text
attempt n:
  run the block; each statement on its domain's local transaction
  ── end of block, or tx.commit(v) ──►
  phase 1: for each domain, in a fixed order:
             PREPARE TRANSACTION 'hql:<id>:<n>:<domain>'
           any refusal ─► ROLLBACK PREPARED for each prepared domain ─► as HQL-0051 (conflict, constraint)
  decide:  journal ← {id, attempt: n, decision: "commit", domains}       ← the commit point
  phase 2: for each domain: COMMIT PREPARED 'hql:<id>:<n>:<domain>'
  journal ← {id, attempt: n, state: "done"}
  ─► ok(v)

recovery (a later run with the same id, before its first attempt):
  decision "commit" recorded    ─► COMMIT PREPARED each still-prepared gid, then "done"
  no decision recorded          ─► ROLLBACK PREPARED each gid with prefix 'hql:<id>:', then run normally
```

- The global transaction identifier MUST be `hql:` followed by the scope's
  `id`, the attempt and the domain's index in the plan, and MUST be shorter
  than 200 bytes, PostgreSQL's limit. An `id` that makes it longer is an
  evaluation failure naming the `id`.
- The decision record is the commit point. Before it, every failure is a
  rollback of every domain, as in HQL-0051. After it, the outcome is
  `commit`, and a failure of phase 2 leaves the scope **in doubt**:
  `err` with `kind = "in_doubt"`, and the next run with the same `id`
  completes it. The program does not decide again.
- A domain whose system has prepared transactions disabled MUST be an
  evaluation failure before the first statement, naming the setting:
  `` `orders` (postgres SHOP_DSN) has max_prepared_transactions = 0 ``.
- Two-phase commit blocks: a prepared transaction holds its locks until it
  is decided. The record says so in `hql check --explain`, and the section
  "Security Considerations" bounds it.

### 9. Staged

`Staged` writes every domain's output so that no reader sees it, then
publishes all of it through one mechanism. The guarantee is about
**visibility to readers that follow the mechanism**, not about the physical
effects, which happen one domain at a time.

```text
attempt n:
  run the block; each effect writes staged output, invisible to readers
  ── end of block, or tx.commit(v) ──►
  journal ← {id, attempt: n, staged: [each domain's staged handle], payloads where needed}
  publish: the mechanism's one act                                         ← the commit point
  finish:  each domain's own publication, in plan order (idempotent)
  journal ← {id, state: "done"};  discard what the mechanism no longer names
  ─► ok(v)
failure before publish ─► discard staged output ─► err, nothing was visible
failure after publish  ─► recovery runs "finish" again from the journal
```

The planner chooses one mechanism per plan. Each mechanism names the
readers it protects.

| Mechanism | Members it covers | Publication act | Readers that see all-or-nothing | Readers that do not |
| --- | --- | --- | --- | --- |
| **outbox** | one `postgres` or `sqlite` domain, plus brokers | the domain's own commit, with the broker messages written as outbox rows in it | every reader of the domain; every consumer of the topic, eventually, once the relay delivers | a consumer reading the topic before the relay has delivered |
| **catalog** | Delta and Iceberg tables of one catalog | the catalog commit | every reader through the catalog | a reader of the files under the catalog |
| **manifest** | snapshot tables, object stores, files | one put of a manifest object naming each output's version | a reader that resolves the manifest, `source(manifest)` | a reader of a table's latest snapshot or of a key directly |

- **outbox** is the one honest way to join a database and a broker. Inside
  the scope, a `sink` to a `kafka` member MUST be written as an `insert` of
  `{effect_id, topic, key, payload}` into the outbox table of the domain's
  `Publishing` endpoint. A relay, `source(outbox) | sink(events)` with the
  idempotent producer, delivers it outside the scope. The publication is
  then atomic with the database commit. The delivery is eventual and,
  with the effect identity in a header, deduplicable by the consumer. The
  plan says both.
- **manifest** requires every covered output to be addressable by version:
  a snapshot id, an object's key under the run, a file's temporary path. A
  member whose output is not addressable is refused under `Staged`.
- A `kafka` member under the manifest mechanism is published last, as one
  producer transaction keyed by the scope's `id`. Its payloads are in the
  journal, since an open producer transaction does not survive the process
  (the section "Capabilities"). A `read_committed` consumer sees the
  messages of one scope together. A consumer that also reads the manifest
  sees them no earlier than the tables.
- A plan that would need two mechanisms is refused, naming both. Two
  publication acts are two commit points, which is a saga.
- An `update` or `delete` under `Staged` is admitted only for a member whose
  staged output is a new version, a snapshot table or a prepared
  transaction. A row changed in place on a live system is visible the moment
  it is written.

### 10. Saga

`Saga` commits each effect in its own domain's local transaction, in
program order, and records each one in the journal. A failure runs the
compensations of the committed effects in reverse order. Retry and recovery
continue from the journal, using effect identities.

#### Compensation is per effect

Each effect of a `Saga` block is one of three classes, decided by the
checker from the statement and its endpoint.

| Class | What it is | How it gets the class |
| --- | --- | --- |
| **compensatable** (C) | an effect with a compensation | derived, when the endpoint is `Compensatable` and the statement is a mutation or a `target` the kind inverts; or written, `compensate { … }` |
| **pivot** (P) | an effect with no compensation | written, `compensate none` |
| **retriable** (R) | an effect after the pivot, retried until it holds | an effect after a pivot over an `IdempotentWrite` endpoint, or one that carries the effect identity |

```text
effect     ::= statement compensation?
compensation ::= "compensate" ( block | "none" )
```

- `compensate` is recognized by position after an effectful statement of a
  `Saga` block, as `set` is recognized after `update`. Anywhere else it is a
  check error: `` `compensate` is a clause of a saga's effect ``.
- A compensation block MUST be effectful statements over participants. It
  MAY read the effect's own result as `it`: the count of a mutation, the
  rows it wrote. It MUST NOT name `tx`.
- A derived compensation is checked as the statement it stands for and
  MUST be shown by `hql check --explain`:

  | Effect | Derived compensation |
  | --- | --- |
  | `insert E rows`, `E` with a key | `delete E \| filter(r => r.key in inserted keys)` |
  | `update E \| filter(p) \| set(f)` | for each changed row, `set` back the before-image, **guarded**: only where the row still holds what `f` wrote |
  | `delete E \| filter(p)` | `insert E` of the deleted rows' before-images |
  | `target(x, file)` | restore the previous content, or remove the file where there was none |

- The before-image of an `update` or `delete` MUST be read in the same
  local transaction as the statement and recorded in the journal. The
  restore is guarded, because a saga holds no locks between effects. A
  row changed by another transaction since then is not overwritten. The
  compensation is then `compensation_failed`, naming the row's key.
- An effect with neither a derived nor a written compensation, and no
  `compensate none`, MUST be a check error naming it:
  `` `sink` to `events` (kafka) has no compensation; write
  `compensate { … }`, or `compensate none` if it is the saga's pivot ``.
- The block MUST read **C\* P? R\***: every compensatable effect, then at
  most one pivot, then only retriable effects. Anything else is a check
  error naming the first effect out of place. This ordering is the reason a saga can always finish: before
  the pivot, a failure can be undone; after it, a failure can be repeated.
- A pure step between two effects may move as any pure step may. No effect
  moves: the order is the program's (HQL-0051, "The scope").

#### The lifecycle

```text
for each effect e_i in program order:
  if journal has e_i done for this id     ─► skip (recovery, or tx.retry())
  run e_i in its domain's local transaction, with its before-image read there
  journal ← {id, effect: i, state: "done", compensation data}       (same transaction where the journal shares the domain)
end of block ─► journal ← {id, state: "done"} ─► ok(v)

failure of e_k, k at or before the pivot:
  for i = k-1 down to 1: run compensation(e_i); journal ← {effect: i, state: "compensated"}
  ─► err(kind = "compensated")                 every compensation held
  ─► err(kind = "compensation_failed")         one did not: it and the rest stay recorded, "compensating"
failure of e_k, k after the pivot:
  ─► err(kind = "in_doubt")                     the next run with this id continues at e_k
```

- An effect's identity MUST be sent with it wherever the endpoint takes one:
  as the outbox row's key, a Kafka or JetStream header, an object key, or
  the `ON CONFLICT` key of an idempotent insert. A crash between an effect
  and its journal record then repeats the effect harmlessly on recovery.
- A step that is not `IdempotentWrite` and whose domain is not the journal's
  can be repeated after a crash. The planner MUST say so in `--explain`
  for each such effect before the pivot. Its compensation runs at most once
  per recorded execution.
- Reads inside a saga's block see committed state only. There is no
  enclosing transaction, so a read after an effect in another domain sees
  what that domain has committed, and nothing isolates it from other
  writers. `isolation` MUST NOT be given with `Saga`.

### 11. The handle per strategy

`tx` keeps HQL-0051's three terminal calls, each of type `Never`, and its
rules on where a call may stand. Their meaning is the strategy's.

| Call | `Atomic` (one domain or 2PC) | `Staged` | `Saga` |
| --- | --- | --- | --- |
| `tx.commit(v)` | prepare, decide, commit every domain | stage, publish, finish | stop; every effect so far has committed |
| `tx.rollback(reason)` | roll back every domain; nothing remains | discard staged output; nothing was visible | compensate the committed effects in reverse order; refused after the pivot |
| `tx.retry()` | roll back and run the block again on fresh transactions | discard and run again | compensate and run again; effects whose journal record survives are not repeated |

- `tx.rollback` after the pivot of a `Saga` block MUST be a check error
  where the checker sees the order, and `err(kind = "in_doubt")` otherwise:
  there is nothing left to undo, only work left to finish.
- `attempts` keeps its meaning: the number of runs of the block in one
  evaluation. Recovery across runs is not an attempt. It is governed by
  `id` and the journal.

### 12. The journal

The journal is an endpoint, so it is checked, configured and granted like
any other ([HQL-0021](../HQL-0021/README.md), "Effects and what a host
grants").

```hql
// proposed: lib/std/io/transaction/mod.hql
/// One record of a coordinated meta-transaction. The coordinator writes
/// it; a program reads it, `source(runs)`, to inspect or report.
type JournalRecord : DataContract = {
    id        : String          // the scope's `id`
    attempt   : Int
    strategy  : String          // "Atomic", "Staged", "Saga"
    effect?   : Int             // the effect's position, for Saga
    domain?   : String
    state     : String          // "staged", "decided", "done", "compensating", "compensated", "failed"
    handle?   : Data            // a gid, a snapshot id, a manifest key, a before-image
    payload?  : Data            // staged output that does not survive the process
}
```

- The journal MUST be `Transactional`, and its records MUST be written in
  one transaction per state change. An endpoint that is not
  `Transactional`, such as a file or a topic, is refused as a journal.
- `payload` and `handle` hold data the scope wrote. They are under the same
  grants and retention as the data itself, and a program that needs to keep
  them for less time deletes `done` records itself.
- The record's fields are this record's contract. A kind MAY add fields,
  since the contract is open, as `TransactionFailure` is.
- Recovery runs only where the program runs again with the same `id`. HQL
  runs no daemon. A host that wants unattended recovery schedules the
  program, and that is the boundary the section "Progression" draws.

### 13. Failures

`TransactionFailure` ([HQL-0051](../HQL-0051/README.md), "The lifecycle")
gains four values of `kind` and one field.

```hql
// proposed: lib/std/io/mod.hql, amending HQL-0051
type TransactionFailure : DataContract = {
    kind     : String
    reason   : String
    attempts : Int
    endpoint? : String
    detail?  : Data
    effects? : List<EffectOutcome>   // a meta-transaction's effects, in program order

    require kind == "rolled_back" or kind == "conflict"
         or kind == "constraint"  or kind == "exhausted"
         or kind == "in_doubt"    or kind == "compensated"
         or kind == "compensation_failed" or kind == "publication_failed"
}

type EffectOutcome : DataContract = {
    effect   : Int
    endpoint : String
    state    : String     // "not_run", "committed", "compensated", "compensation_failed", "pending"
}
```

| What happened | Side | Where it goes |
| --- | --- | --- |
| phase 2 of 2PC, or a step after a saga's pivot, did not complete | data | `err`, `kind = "in_doubt"`; the next run with the same `id` completes it |
| a saga failed before its pivot, and every compensation held | data | `err`, `kind = "compensated"` |
| a compensation failed: a guarded restore found a changed row, the endpoint refused | data | `err`, `kind = "compensation_failed"`, with `effects` |
| the publication act of `Staged` was refused | data | `err`, `kind = "publication_failed"`; nothing was visible |
| a strategy the members cannot have, an effect out of the C\* P? R\* order, an effect with no compensation, a `Set<Endpoint>` in `on`, `journal` without `id` | diagnostic, `check` | `invalid-argument` |
| `isolation` with `Saga`; `tx.rollback` after a pivot | diagnostic, `check` | `invalid-argument` |
| prepared transactions disabled; the journal unreachable at the start | diagnostic, `execute`, fatal, before the first effect | `io-failure` |
| the journal unreachable after the first effect | diagnostic, `execute`, fatal | `io-failure`; the domains' own state is what the next run recovers from |

The last row is the hard case, and the record does not hide it. A
coordinator that cannot write its log cannot promise anything more. Every
plan therefore places a journal write before each step that a later run needs to know
about, so the next run recovers to a defined state.

## The worked example

The order completion across four endpoints, under each strategy.

```hql
// proposed; delta(...) is design direction
import io.db.postgres
import io.msg.kafka
import io.store.delta

type OrderChanged { order_id : Int, status : String }

orders    = postgres(dsn_env = "SHOP_DSN", table = "orders")
analytics = delta(catalog_env = "LAKE_CATALOG", table = "silver.orders")
events    = kafka(topic = "orders.changed", brokers_env = "KAFKA_BROKERS")
archive   = file("archive/orders/42.json")
runs      = postgres(dsn_env = "SHOP_DSN", table = "hql_journal")

destinations = {orders, analytics, events, archive}
    // Participants{orders: Postgres, analytics: Delta, events: Kafka, archive: File}

outcome =
    with transaction(strategy = Saga, id = "order-42-completed", journal = runs) tx
    on destinations {
        update orders
        | filter(o => o.id == 42)
        | set(o => {status: "completed"})
                                                   // C: derived, guarded restore
        insert analytics {order_id: 42, status: "completed"}
                                                   // C: derived, delete by key
        source(orders)
        | filter(o => o.id == 42)
        | map(o => OrderChanged(order_id = o.id, status = o.status))
        | sink(events, Json)
          compensate none                          // P: once published, an event is read
        source(orders)
        | filter(o => o.id == 42)
        | target(archive, Json)                    // R: a put under one key, idempotent
    }

match outcome {
    ok(_)  => "order 42 completed everywhere",
    err(f) => fmt("order 42: {} ({})", f.kind, f.reason),
}
```

- `update orders` and `insert analytics` are compensatable. If the event
  cannot be published, the analytics row is deleted and the order's status
  is restored, provided nobody has changed it since.
- The event is the pivot. After it, the saga only moves forward: the archive
  write is retried, now or on the next run with `order-42-completed`, until
  it holds. `id` makes the archive put and the journal records idempotent.
- The same set without `strategy` is the refusal of the section "The
  planner": four commit domains, and `events` cannot prepare.
- A program that wants the event to be all-or-nothing with the order row
  writes `strategy = Staged` over `{orders, events}`. The planner chooses the
  outbox, so the event row commits with the order and the relay delivers it.
  The analytics table and the archive would then go in a second scope, or
  in the same saga after the outbox, as its pivot.

Under `--explain` the saga reads:

```text
scope at order.hql:16  strategy Saga  id "order-42-completed"  journal runs (postgres SHOP_DSN)
  domains  orders (postgres SHOP_DSN, shares the journal), analytics (delta LAKE_CATALOG),
           events (kafka), archive (file)
  1 C  update orders           compensation: guarded restore of status    journal in-transaction
  2 C  insert analytics        compensation: delete where order_id = 42    may repeat after a crash; keyed, idempotent
  3 P  sink events             no compensation                             effect id in header hql-effect-id
  4 R  target archive          retried forward                             put under one key
  readers: no reader sees a single moment where all four hold; each sees its own domain's commit
```

## Progression

The record places HQL's transactions on a line, so that each step keeps the
syntax of the one before it.

1. **SQL parity**: one relational system, [HQL-0051](../HQL-0051/README.md).
2. **Multi-table domains**: a catalog with catalog commits as one more commit
   domain under HQL-0051's rule, with no coordinator, as soon as a Delta or
   Iceberg kind exists.
3. **Heterogeneous strategies**: this record. `Atomic` across prepared
   domains, `Staged` through one publication mechanism, `Saga` with
   compensations, all checked by the planner.
4. **Durable execution**: a later record. The journal becomes the history of
   a long-running program, recovery runs without the program being started
   again, and waits and timers enter through a backend that schedules in
   time. This overlaps with workflow systems such as Temporal. The difference that would
   justify it is that the workflow, its effects and every endpoint's
   capabilities are typed HQL, and the guarantee is checked before the first
   step.

What the line does not reach is universal ACID. Its long-term proposition is
narrower and more useful: one language for writing down which consistency a
set of endpoints can jointly give, and a checker that refuses to promise
more.

## Backwards Compatibility

- Every scope that HQL-0051 admits is admitted unchanged, with the same
  meaning: no `strategy` is `Atomic`, and one commit domain needs no
  journal.
- HQL-0051's refusal of a set of two kinds stays, under `Atomic`, for any
  pair whose kinds cannot both prepare. The diagnostic gains the planner's
  last line, which names the strategy the set could have.
- `CoordinationStrategy`, `Atomic`, `Staged`, `Saga`, `PreparedCommit`,
  `StagedWrite`, `IdempotentWrite`, `Compensatable`, `Publishing`,
  `JournalRecord` and `EffectOutcome` are new names in the prelude. A
  program binding one shadows it.
- `compensate` is recognized by position after an effect in a `Saga` block,
  where nothing is admitted today.
- `TransactionFailure` gains `kind` values and an optional field. It is an
  open contract, and a program that reads the four stable fields is
  unaffected.

## Security Considerations

- The journal holds data the scope wrote, including before-images and
  payloads. It is an endpoint like any other: it names no secret, a
  constructor names an environment variable, and it is granted or refused
  by the host.
- A compensation runs with the same grants as the effect it undoes. A host
  that grants a write and refuses its compensation refuses the scope at
  check time, never halfway through.
- A prepared transaction holds locks until it is decided, and an
  undecided one blocks `VACUUM`. The coordinator MUST decide every
  transaction it prepared before the evaluation ends, or record it as
  `in_doubt`. `hql check --explain` MUST list the prepared domains, so an
  operator knows what `pg_prepared_xacts` may hold.
- `hql render` refuses every scope, as HQL-0051 does. A meta-transaction
  is never run while rendering a document.
- The effect identity is derived from `id`, position and attempt. It
  MUST NOT contain row data, since it is sent as a header or a key.
- A compensation written by the program is ordinary code with ordinary
  effects. The checker verifies that it is well-typed, and does not verify that it is
  a correct inverse. `--explain` labels it "written", beside the
  derived ones.

## Deployment / Activation

Each phase is committed with its tests before the next, and each is usable
alone. Every phase follows HQL-0051's phase 3 (the scope, one attempt).

1. **Types and the planner, check only.** The five capabilities, the
   strategy types, `Participants` typing, the planner and
   `hql check --explain`. Every scope over more than one domain is checked
   and refused at evaluation with "planned, not yet executed". This phase
   alone tells a program what it could have.
2. **`Saga` over databases and files.** The journal, `id`, effect
   identities, derived compensations for `postgres`, `sqlite` and `file`,
   written compensations, the C\* P? R\* rule, recovery on the next run.
3. **`Staged` by outbox.** One `postgres` domain plus `kafka`, the outbox row,
   and a relay example in `hql-demo`.
4. **`Atomic` by two-phase commit.** Two `postgres` domains, the decision
   record, in-doubt recovery, the `max_prepared_transactions` check.
5. **Lakehouse and object stores.** The catalog and manifest mechanisms as
   the Delta, Iceberg and `s3` kinds of [HQL-0048](../HQL-0048/README.md)
   land.
6. **The book.** A page in the OLTP chapter, the order across four systems,
   showing all three strategies and the refusal.

## Reference Implementation

- `lib/std/io/mod.hql` for the capabilities, `CoordinationStrategy` and its
  three types, and the amended `TransactionFailure` with `EffectOutcome`;
  `lib/std/io/transaction/mod.hql` for `JournalRecord`; each kind's module
  for its new supertypes.
- `src/parser.rs`: `strategy`, `id` and `journal` in the scope's head; the
  `compensate` clause; `src/ast.rs` for its node.
- `src/checker.rs`: the `Participants` type and its escape rule; the
  refusal of an erased set; the C\* P? R\* order; `isolation` with `Saga`;
  `tx.rollback` after a pivot.
- `src/coordination.rs` (new): the planner, as a pure function from
  members, kinds and effects to a plan or a diagnostic, and the `--explain`
  rendering.
- `src/coordinator.rs` (new): the runtime of each plan, with 2PC, publication
  and saga steps, and recovery from the journal.
- `src/endpoints.rs`: each kind's commit-domain rule and capabilities;
  `src/postgres.rs`: `PREPARE TRANSACTION`, `COMMIT PREPARED`,
  `ROLLBACK PREPARED`, before-images, the outbox insert; the Kafka sink's
  effect-identity header.
- `src/evaluation.rs`: the scope dispatches on its plan; HQL-0051's attempt
  loop stays the `native` plan.

## Test Plan

Proposed cases under `examples/transactions/coordination/`, each
`status: proposed` and `proposal: HQL-0052`, promoted as each phase lands.
`tests/coordination.rs` tests the planner with no network, since the planner is a
pure function.

Unit and integration tests MUST include:

- the planner: one domain under each strategy giving the `native` plan;
  `postgres` + `kafka` under `Atomic` refused naming `events` and offering
  `Staged`; two `postgres` domains under `Atomic` with a journal giving
  `two_phase`, and without one refused naming `journal`; `postgres` +
  `kafka` under `Staged` giving `outbox`; a plan needing two mechanisms
  refused naming both; the worked example under `Saga` giving the plan
  shown, byte for byte, by `--explain`;
- typing: `destinations` typed `Participants{…}` with four kinds; the
  binding passed to a function, iterated, or held in a list, each refused
  naming the scope; a `Set<Endpoint>` in `on` refused naming the erasure;
- the saga order: a compensatable effect after the pivot refused; two
  pivots refused; a `kafka` sink with no clause refused naming
  `compensate`; `compensate` outside a saga refused; `tx.rollback` after the
  pivot refused;
- over SQLite and files, in `tests/`, with a fault injected after each
  effect in turn: compensations run in reverse order and the tables and
  file end as they began; a guarded restore over a row changed by a second
  connection gives `compensation_failed` with that effect's state; a crash
  after the pivot followed by a second run with the same `id` finishing the
  remaining effects without repeating a done one; a second run of a
  finished `id` doing nothing;
- over PostgreSQL 17 in `hql-demo`: 2PC across two databases committed;
  refused in phase 1 by the second database and leaving nothing; a process
  killed after the decision record, recovered by the next run with
  `COMMIT PREPARED`; killed before it, recovered by `ROLLBACK PREPARED`;
  `max_prepared_transactions = 0` refused before the first statement;
- over PostgreSQL 17 and Kafka in `hql-demo`: the outbox scope committing
  the order row and its event row together, the relay delivering it once
  with its effect identity, and a rolled-back scope delivering nothing.

```bash
cargo test --locked --test coordination --test transactions --test corpus
cargo clippy --locked --all-targets -- -D warnings
../hql-demo/run-all.sh
```

## Open Questions

- Should the outbox be written by the program as an `insert`, or should a
  `sink` to a broker inside an outbox plan be rewritten by the planner? A
  rewrite reads better, but it makes a `sink` mean two things.
- Is `compensate none` the right spelling of a pivot, or should the pivot be
  a word of its own, `pivot sink(…)`, which names the role and not the absence?
- What is the journal's physical layout: one table per program, one per
  scope, or one shared with a `program` column? And who removes `done`
  records?
- How does a `Staged` reader name the manifest: `source(manifest)` giving
  the tables at their published versions, or an `as_of` argument on
  `source`? The latter belongs to the Delta and Iceberg kinds, which are
  design direction.
- Should a derived compensation of a Delta `update` rewrite the rows to
  their before-images, or restore the table version where no other commit
  has happened since? The second is cheaper and is wrong under any
  concurrent writer.
- With no clock, a prepared transaction or a saga after its pivot waits for
  the next run. Should a scope take a bound, `decide_within = 5m`, enforced
  by the host rather than by evaluation?
- Should `Staged` admit `update` over a live table by staging a copy, or
  stay limited to outputs that are new versions?
- A meta-transaction inside a meta-transaction, and a saga step that is
  itself an `Atomic` scope over one domain. The second looks useful and is
  what a step already is. Should it be written explicitly?
- Should `IdempotentWrite` for `kafka` require the transactional id to be
  derived from `id`, so that idempotency spans producer sessions, or stay
  within one session as the kind gives it?

## Changelog

- 2026-11-13: drafted
